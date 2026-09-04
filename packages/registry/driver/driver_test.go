package driver

import (
	"container/list"
	"context"
	"fmt"
	"io"
	"net/http"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/cplane/cplane/registry/tenant"
	storagedriver "github.com/distribution/distribution/v3/registry/storage/driver"
)

func TestDriverCacheRevisionExpiryAndEviction(t *testing.T) {
	now := time.Unix(1_700_000_000, 0)
	var builds atomic.Int32
	d := testDriver(1, time.Minute, func(context.Context, tenant.Metadata) (storagedriver.StorageDriver, error) {
		builds.Add(1)
		return noopDriver{}, nil
	})
	d.now = func() time.Time { return now }

	selectDriver(t, d, metadata("org-1", "revision-1"))
	selectDriver(t, d, metadata("org-1", "revision-1"))
	if got := builds.Load(); got != 1 {
		t.Fatalf("same revision built %d drivers, want 1", got)
	}

	selectDriver(t, d, metadata("org-1", "revision-2"))
	if got := builds.Load(); got != 2 {
		t.Fatalf("revision change built %d drivers, want 2", got)
	}

	now = now.Add(2 * time.Minute)
	selectDriver(t, d, metadata("org-1", "revision-2"))
	if got := builds.Load(); got != 3 {
		t.Fatalf("expired entry built %d drivers, want 3", got)
	}

	selectDriver(t, d, metadata("org-2", "revision-1"))
	selectDriver(t, d, metadata("org-1", "revision-2"))
	if got := builds.Load(); got != 5 {
		t.Fatalf("evicted entry built %d drivers, want 5", got)
	}
}

func TestConcurrentMissesUseSingleflight(t *testing.T) {
	var builds atomic.Int32
	started := make(chan struct{})
	release := make(chan struct{})
	d := testDriver(256, time.Minute, func(context.Context, tenant.Metadata) (storagedriver.StorageDriver, error) {
		if builds.Add(1) == 1 {
			close(started)
		}
		<-release
		return noopDriver{}, nil
	})

	const callers = 16
	var wg sync.WaitGroup
	wg.Add(callers)
	errors := make(chan error, callers)
	for range callers {
		go func() {
			defer wg.Done()
			_, err := d.selected(tenant.WithMetadata(context.Background(), metadata("org-1", "revision-1")))
			errors <- err
		}()
	}
	<-started
	close(release)
	wg.Wait()
	close(errors)
	for err := range errors {
		if err != nil {
			t.Fatal(err)
		}
	}
	if got := builds.Load(); got != 1 {
		t.Fatalf("concurrent miss built %d drivers, want 1", got)
	}
}

func TestRepositoryOperationsTranslateLogicalAndPhysicalPaths(t *testing.T) {
	logical := repositoryRoot + "acme/11111111-1111-4111-8111-111111111111/api"
	physical := repositoryRoot + "22222222-2222-4222-8222-222222222222"
	recorder := &pathDriver{}
	d := testDriver(1, time.Minute, func(context.Context, tenant.Metadata) (storagedriver.StorageDriver, error) {
		return recorder, nil
	})
	meta := metadata("org-1", "revision-1")
	meta.RepositoryName = "acme/11111111-1111-4111-8111-111111111111/api"
	meta.RepositoryID = "22222222-2222-4222-8222-222222222222"
	ctx := tenant.WithMetadata(context.Background(), meta)

	_, _ = d.GetContent(ctx, logical+"/_manifests/revisions/latest")
	_ = d.PutContent(ctx, logical+"/_uploads/data", nil)
	_, _ = d.Stat(ctx, logical+"/_layers/sha256")
	paths, _ := d.List(ctx, logical+"/_manifests")
	_ = d.Move(ctx, logical+"/from", logical+"/to")
	_ = d.Delete(ctx, logical+"/_uploads/old")
	var walked string
	_ = d.Walk(ctx, logical, func(info storagedriver.FileInfo) error { walked = info.Path(); return nil })

	wantCalls := []string{
		"get:" + physical + "/_manifests/revisions/latest",
		"put:" + physical + "/_uploads/data",
		"stat:" + physical + "/_layers/sha256",
		"list:" + physical + "/_manifests",
		"move:" + physical + "/from:" + physical + "/to",
		"delete:" + physical + "/_uploads/old",
		"walk:" + physical,
	}
	if fmt.Sprint(recorder.calls) != fmt.Sprint(wantCalls) {
		t.Fatalf("calls=%v, want %v", recorder.calls, wantCalls)
	}
	if paths[0] != logical+"/_manifests/item" || walked != logical+"/walked" {
		t.Fatalf("reverse mapping list=%v walk=%q", paths, walked)
	}
}

func testDriver(capacity int, ttl time.Duration, build driverBuilder) *Driver {
	return &Driver{
		capacity: capacity,
		idleTTL:  ttl,
		entries:  make(map[string]*cacheEntry),
		lru:      listForTest(),
		build:    build,
		now:      time.Now,
	}
}

func listForTest() *list.List { return list.New() }

func metadata(organizationID, revision string) tenant.Metadata {
	return tenant.Metadata{
		OrganizationID:     organizationID,
		StorageRevision:    revision,
		AccessKeyID:        "access-key",
		SecretAccessKey:    "secret-key",
		BucketName:         "registry",
		StorageEndpointURL: "http://storage:8081",
	}
}

func selectDriver(t *testing.T, d *Driver, metadata tenant.Metadata) {
	t.Helper()
	if _, err := d.selected(tenant.WithMetadata(context.Background(), metadata)); err != nil {
		t.Fatal(err)
	}
}

type noopDriver struct{}

type pathDriver struct {
	noopDriver
	calls []string
}

func (driver *pathDriver) GetContent(_ context.Context, path string) ([]byte, error) {
	driver.calls = append(driver.calls, "get:"+path)
	return nil, nil
}
func (driver *pathDriver) PutContent(_ context.Context, path string, _ []byte) error {
	driver.calls = append(driver.calls, "put:"+path)
	return nil
}
func (driver *pathDriver) Stat(_ context.Context, path string) (storagedriver.FileInfo, error) {
	driver.calls = append(driver.calls, "stat:"+path)
	return storagedriver.FileInfoInternal{FileInfoFields: storagedriver.FileInfoFields{Path: path}}, nil
}
func (driver *pathDriver) List(_ context.Context, path string) ([]string, error) {
	driver.calls = append(driver.calls, "list:"+path)
	return []string{path + "/item"}, nil
}
func (driver *pathDriver) Move(_ context.Context, source, destination string) error {
	driver.calls = append(driver.calls, "move:"+source+":"+destination)
	return nil
}
func (driver *pathDriver) Delete(_ context.Context, path string) error {
	driver.calls = append(driver.calls, "delete:"+path)
	return nil
}
func (driver *pathDriver) Walk(_ context.Context, path string, walk storagedriver.WalkFn, _ ...func(*storagedriver.WalkOptions)) error {
	driver.calls = append(driver.calls, "walk:"+path)
	return walk(storagedriver.FileInfoInternal{FileInfoFields: storagedriver.FileInfoFields{Path: path + "/walked"}})
}

func (noopDriver) Name() string                                                 { return "noop" }
func (noopDriver) GetContent(context.Context, string) ([]byte, error)           { return nil, nil }
func (noopDriver) PutContent(context.Context, string, []byte) error             { return nil }
func (noopDriver) Reader(context.Context, string, int64) (io.ReadCloser, error) { return nil, nil }
func (noopDriver) Writer(context.Context, string, bool) (storagedriver.FileWriter, error) {
	return nil, nil
}
func (noopDriver) Stat(context.Context, string) (storagedriver.FileInfo, error) { return nil, nil }
func (noopDriver) List(context.Context, string) ([]string, error)               { return nil, nil }
func (noopDriver) Move(context.Context, string, string) error                   { return nil }
func (noopDriver) Delete(context.Context, string) error                         { return nil }
func (noopDriver) RedirectURL(*http.Request, string) (string, error)            { return "", nil }
func (noopDriver) Walk(context.Context, string, storagedriver.WalkFn, ...func(*storagedriver.WalkOptions)) error {
	return nil
}
