package driver

import (
	"container/list"
	"context"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"sync"
	"time"

	"github.com/cplane/cplane/registry/telemetry"
	"github.com/cplane/cplane/registry/tenant"
	storagedriver "github.com/distribution/distribution/v3/registry/storage/driver"
	"github.com/distribution/distribution/v3/registry/storage/driver/factory"
	s3aws "github.com/distribution/distribution/v3/registry/storage/driver/s3-aws"
	"golang.org/x/sync/singleflight"
)

const Name = "cplane"

type Factory struct{}

type driverBuilder func(context.Context, tenant.Metadata) (storagedriver.StorageDriver, error)

func (Factory) Create(_ context.Context, parameters map[string]any) (storagedriver.StorageDriver, error) {
	capacity := 256
	if raw := parameters["cachecapacity"]; raw != nil {
		parsed, err := strconv.Atoi(fmt.Sprint(raw))
		if err != nil || parsed < 1 {
			return nil, fmt.Errorf("cachecapacity must be a positive integer")
		}
		capacity = parsed
	}
	ttl := 15 * time.Minute
	if raw := parameters["cachettl"]; raw != nil {
		parsed, err := time.ParseDuration(fmt.Sprint(raw))
		if err != nil || parsed <= 0 {
			return nil, fmt.Errorf("cachettl must be a positive duration")
		}
		ttl = parsed
	}
	return &Driver{
		capacity:        capacity,
		idleTTL:         ttl,
		entries:         make(map[string]*cacheEntry),
		lru:             list.New(),
		controlPlaneURL: fmt.Sprint(parameters["controlplaneurl"]),
		serviceToken:    fmt.Sprint(parameters["servicetoken"]),
		http:            &http.Client{Timeout: 5 * time.Second},
		build:           buildS3Driver,
		now:             time.Now,
	}, nil
}

type cacheEntry struct {
	organizationID string
	revision       string
	driver         storagedriver.StorageDriver
	lastUsed       time.Time
	element        *list.Element
}

type Driver struct {
	mu              sync.Mutex
	capacity        int
	idleTTL         time.Duration
	entries         map[string]*cacheEntry
	lru             *list.List
	loads           singleflight.Group
	controlPlaneURL string
	serviceToken    string
	http            *http.Client
	build           driverBuilder
	now             func() time.Time
}

var _ storagedriver.StorageDriver = (*Driver)(nil)

func (d *Driver) Name() string { return Name }

func (d *Driver) selected(ctx context.Context) (storagedriver.StorageDriver, error) {
	metadata, ok := tenant.FromContext(ctx)
	if !ok {
		return nil, storagedriver.Error{DriverName: Name, Detail: fmt.Errorf("tenant context is required")}
	}
	now := d.now()
	d.mu.Lock()
	if entry := d.entries[metadata.OrganizationID]; entry != nil {
		if entry.revision == metadata.StorageRevision && now.Sub(entry.lastUsed) <= d.idleTTL {
			telemetry.CacheEvents.WithLabelValues("hit").Inc()
			entry.lastUsed = now
			d.lru.MoveToFront(entry.element)
			driver := entry.driver
			d.mu.Unlock()
			return driver, nil
		}
		if entry.revision != metadata.StorageRevision {
			telemetry.CacheEvents.WithLabelValues("revision_invalidated").Inc()
		} else {
			telemetry.CacheEvents.WithLabelValues("expired").Inc()
		}
		d.removeLocked(entry)
	}
	d.mu.Unlock()
	telemetry.CacheEvents.WithLabelValues("miss").Inc()

	loaded, err, _ := d.loads.Do(metadata.OrganizationID+":"+metadata.StorageRevision, func() (any, error) {
		d.mu.Lock()
		now := d.now()
		if entry := d.entries[metadata.OrganizationID]; entry != nil && entry.revision == metadata.StorageRevision && now.Sub(entry.lastUsed) <= d.idleTTL {
			entry.lastUsed = now
			d.lru.MoveToFront(entry.element)
			driver := entry.driver
			d.mu.Unlock()
			return driver, nil
		}
		d.mu.Unlock()

		driver, err := d.build(ctx, metadata)
		if err != nil {
			return nil, err
		}
		d.mu.Lock()
		if old := d.entries[metadata.OrganizationID]; old != nil {
			d.removeLocked(old)
		}
		entry := &cacheEntry{organizationID: metadata.OrganizationID, revision: metadata.StorageRevision, driver: driver, lastUsed: d.now()}
		entry.element = d.lru.PushFront(entry)
		d.entries[metadata.OrganizationID] = entry
		for len(d.entries) > d.capacity {
			telemetry.CacheEvents.WithLabelValues("evicted").Inc()
			d.removeLocked(d.lru.Back().Value.(*cacheEntry))
		}
		d.mu.Unlock()
		return driver, nil
	})
	if err != nil {
		return nil, storagedriver.Error{DriverName: Name, Detail: err}
	}
	return loaded.(storagedriver.StorageDriver), nil
}

func buildS3Driver(ctx context.Context, metadata tenant.Metadata) (storagedriver.StorageDriver, error) {
	endpoint, err := url.Parse(metadata.StorageEndpointURL)
	if err != nil || endpoint.Host == "" {
		return nil, fmt.Errorf("invalid Storage endpoint")
	}
	return s3aws.FromParameters(ctx, map[string]any{
		"accesskey":      metadata.AccessKeyID,
		"secretkey":      metadata.SecretAccessKey,
		"region":         "us-east-1",
		"regionendpoint": metadata.StorageEndpointURL,
		"bucket":         metadata.BucketName,
		"secure":         endpoint.Scheme == "https",
		"v4auth":         true,
		"forcepathstyle": true,
		"encrypt":        false,
		"useragent":      "cplane-registry",
	})
}

func NewForTenant(ctx context.Context, metadata tenant.Metadata) (storagedriver.StorageDriver, error) {
	return buildS3Driver(ctx, metadata)
}

func (d *Driver) removeLocked(entry *cacheEntry) {
	delete(d.entries, entry.organizationID)
	d.lru.Remove(entry.element)
}

func (d *Driver) GetContent(ctx context.Context, path string) ([]byte, error) {
	started := time.Now()
	driver, err := d.selected(ctx)
	if err != nil {
		observeOperation("get_content", started, err)
		return nil, err
	}
	content, err := driver.GetContent(ctx, path)
	observeOperation("get_content", started, err)
	if err == nil {
		telemetry.Bytes.WithLabelValues("download").Add(float64(len(content)))
	}
	return content, err
}
func (d *Driver) PutContent(ctx context.Context, path string, content []byte) error {
	started := time.Now()
	driver, err := d.selected(ctx)
	if err != nil {
		observeOperation("put_content", started, err)
		return err
	}
	err = driver.PutContent(ctx, path, content)
	observeOperation("put_content", started, err)
	if err == nil {
		telemetry.Bytes.WithLabelValues("upload").Add(float64(len(content)))
	}
	return err
}
func (d *Driver) Reader(ctx context.Context, path string, offset int64) (io.ReadCloser, error) {
	started := time.Now()
	driver, err := d.selected(ctx)
	if err != nil {
		observeOperation("reader", started, err)
		return nil, err
	}
	reader, err := driver.Reader(ctx, path, offset)
	observeOperation("reader", started, err)
	if err != nil {
		return nil, err
	}
	return &meteredReadCloser{ReadCloser: reader}, nil
}
func (d *Driver) Writer(ctx context.Context, path string, appendMode bool) (storagedriver.FileWriter, error) {
	started := time.Now()
	driver, err := d.selected(ctx)
	if err != nil {
		observeOperation("writer", started, err)
		return nil, err
	}
	writer, err := driver.Writer(ctx, path, appendMode)
	observeOperation("writer", started, err)
	if err != nil {
		return nil, err
	}
	return &meteredFileWriter{FileWriter: writer}, nil
}
func (d *Driver) Stat(ctx context.Context, path string) (storagedriver.FileInfo, error) {
	started := time.Now()
	if path == "/" {
		if _, ok := tenant.FromContext(ctx); !ok {
			if err := d.ready(ctx); err != nil {
				return nil, err
			}
			return nil, storagedriver.PathNotFoundError{Path: path, DriverName: Name}
		}
	}
	driver, err := d.selected(ctx)
	if err != nil {
		observeOperation("stat", started, err)
		return nil, err
	}
	info, err := driver.Stat(ctx, path)
	observeOperation("stat", started, err)
	return info, err
}
func (d *Driver) List(ctx context.Context, path string) ([]string, error) {
	started := time.Now()
	driver, err := d.selected(ctx)
	if err != nil {
		observeOperation("list", started, err)
		return nil, err
	}
	paths, err := driver.List(ctx, path)
	observeOperation("list", started, err)
	return paths, err
}
func (d *Driver) Move(ctx context.Context, sourcePath, destPath string) error {
	started := time.Now()
	driver, err := d.selected(ctx)
	if err != nil {
		observeOperation("move", started, err)
		return err
	}
	err = driver.Move(ctx, sourcePath, destPath)
	observeOperation("move", started, err)
	return err
}
func (d *Driver) Delete(ctx context.Context, path string) error {
	started := time.Now()
	driver, err := d.selected(ctx)
	if err != nil {
		observeOperation("delete", started, err)
		return err
	}
	err = driver.Delete(ctx, path)
	observeOperation("delete", started, err)
	return err
}
func (d *Driver) RedirectURL(r *http.Request, path string) (string, error) {
	started := time.Now()
	driver, err := d.selected(r.Context())
	if err != nil {
		telemetry.Redirects.WithLabelValues("error").Inc()
		observeOperation("redirect", started, err)
		return "", err
	}
	redirect, err := driver.RedirectURL(r, path)
	result := "success"
	if err != nil {
		result = "error"
	} else if redirect == "" {
		result = "disabled"
	}
	telemetry.Redirects.WithLabelValues(result).Inc()
	observeOperation("redirect", started, err)
	return redirect, err
}
func (d *Driver) Walk(ctx context.Context, path string, f storagedriver.WalkFn, options ...func(*storagedriver.WalkOptions)) error {
	started := time.Now()
	driver, err := d.selected(ctx)
	if err != nil {
		observeOperation("walk", started, err)
		return err
	}
	err = driver.Walk(ctx, path, f, options...)
	observeOperation("walk", started, err)
	return err
}

type meteredReadCloser struct{ io.ReadCloser }

func (reader *meteredReadCloser) Read(buffer []byte) (int, error) {
	count, err := reader.ReadCloser.Read(buffer)
	telemetry.Bytes.WithLabelValues("download").Add(float64(count))
	return count, err
}

type meteredFileWriter struct{ storagedriver.FileWriter }

func (writer *meteredFileWriter) Write(buffer []byte) (int, error) {
	count, err := writer.FileWriter.Write(buffer)
	telemetry.Bytes.WithLabelValues("upload").Add(float64(count))
	return count, err
}

func observeOperation(operation string, started time.Time, err error) {
	result := "success"
	if err != nil {
		result = "error"
	}
	telemetry.OperationLatency.WithLabelValues(operation, result).Observe(time.Since(started).Seconds())
}

func (d *Driver) ready(ctx context.Context) error {
	if d.controlPlaneURL == "" || d.serviceToken == "" {
		return fmt.Errorf("control-plane resolver is not configured")
	}
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, d.controlPlaneURL+"/health", nil)
	if err != nil {
		return err
	}
	response, err := d.http.Do(request)
	if err != nil {
		return err
	}
	defer response.Body.Close()
	if response.StatusCode/100 != 2 {
		return fmt.Errorf("control-plane readiness returned %s", response.Status)
	}
	return nil
}

func Register() { factory.Register(Name, Factory{}) }
