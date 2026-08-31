package garbagecollection

import (
	"context"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/cplane/cplane/registry/driver"
	"github.com/cplane/cplane/registry/tenant"
	"github.com/distribution/distribution/v3/registry/storage"
	storagedriver "github.com/distribution/distribution/v3/registry/storage/driver"
	"github.com/sirupsen/logrus"
)

type Report struct {
	BytesBefore int64 `json:"bytes_before"`
	BytesAfter  int64 `json:"bytes_after"`
}

type measurement struct {
	bytes           int64
	blobBytes       int64
	uploadBytes     int64
	repositoryBytes int64
	otherBytes      int64
	objects         int64
}

func Run(
	ctx context.Context,
	client *http.Client,
	controlPlaneURL string,
	serviceToken string,
	organizationID string,
	jobID string,
) (Report, error) {
	runStarted := time.Now()
	fields := logrus.Fields{"organization_id": organizationID}
	if jobID != "" {
		fields["job_id"] = jobID
	}
	logger := logrus.WithFields(fields)
	resolveStarted := time.Now()
	logger.Info("resolving managed Registry metadata for garbage collection")
	metadata, err := tenant.Resolve(
		ctx,
		client,
		controlPlaneURL,
		serviceToken,
		organizationID,
	)
	if err != nil {
		return Report{}, fmt.Errorf("resolve managed Registry: %w", err)
	}
	logger.WithFields(logrus.Fields{
		"duration_ms": time.Since(resolveStarted).Milliseconds(),
		"status":      metadata.Status,
	}).Info("managed Registry metadata resolved for garbage collection")
	if metadata.Status != "maintenance" {
		return Report{}, fmt.Errorf("managed Registry must be in maintenance before garbage collection")
	}
	storageStarted := time.Now()
	logger.Info("constructing managed Registry storage for garbage collection")
	storageDriver, err := driver.NewForTenant(ctx, metadata)
	if err != nil {
		return Report{}, fmt.Errorf("construct managed Registry storage driver: %w", err)
	}
	namespace, err := storage.NewRegistry(ctx, storageDriver)
	if err != nil {
		return Report{}, fmt.Errorf("construct managed Registry namespace: %w", err)
	}
	logger.WithField("duration_ms", time.Since(storageStarted).Milliseconds()).Info("managed Registry storage ready for garbage collection")

	measureBeforeStarted := time.Now()
	logger.Info("measuring managed Registry storage before garbage collection")
	before, err := measure(ctx, storageDriver)
	if err != nil {
		return Report{}, fmt.Errorf("measure managed Registry storage: %w", err)
	}
	logger.WithFields(logrus.Fields{
		"blob_bytes":       before.blobBytes,
		"bytes":            before.bytes,
		"duration_ms":      time.Since(measureBeforeStarted).Milliseconds(),
		"objects":          before.objects,
		"other_bytes":      before.otherBytes,
		"repository_bytes": before.repositoryBytes,
		"upload_bytes":     before.uploadBytes,
	}).Info("managed Registry storage measured before garbage collection")

	collectStarted := time.Now()
	logger.Info("managed Registry mark-and-sweep starting")
	if err := storage.MarkAndSweep(ctx, storageDriver, namespace, storage.GCOpts{RemoveUntagged: true}); err != nil {
		return Report{}, fmt.Errorf("garbage collect managed Registry: %w", err)
	}
	logger.WithField("duration_ms", time.Since(collectStarted).Milliseconds()).Info("managed Registry mark-and-sweep completed")

	measureAfterStarted := time.Now()
	logger.Info("measuring managed Registry storage after garbage collection")
	after, err := measure(ctx, storageDriver)
	if err != nil {
		return Report{}, fmt.Errorf("measure managed Registry storage: %w", err)
	}
	logger.WithFields(logrus.Fields{
		"blob_bytes":       after.blobBytes,
		"bytes":            after.bytes,
		"bytes_reclaimed":  before.bytes - after.bytes,
		"duration_ms":      time.Since(measureAfterStarted).Milliseconds(),
		"objects":          after.objects,
		"other_bytes":      after.otherBytes,
		"repository_bytes": after.repositoryBytes,
		"total_ms":         time.Since(runStarted).Milliseconds(),
		"upload_bytes":     after.uploadBytes,
	}).Info("managed Registry garbage collection finished")
	return Report{BytesBefore: before.bytes, BytesAfter: after.bytes}, nil
}

func measure(ctx context.Context, storageDriver storagedriver.StorageDriver) (measurement, error) {
	var result measurement
	err := storageDriver.Walk(ctx, "/", func(info storagedriver.FileInfo) error {
		size := info.Size()
		result.bytes += size
		result.objects++
		switch path := info.Path(); {
		case strings.Contains(path, "/_uploads/"):
			result.uploadBytes += size
		case strings.Contains(path, "/blobs/"):
			result.blobBytes += size
		case strings.Contains(path, "/repositories/"):
			result.repositoryBytes += size
		default:
			result.otherBytes += size
		}
		return nil
	})
	if err != nil {
		return measurement{}, err
	}
	return result, nil
}
