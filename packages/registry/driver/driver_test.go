package driver

import (
	"container/list"
	"context"
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
