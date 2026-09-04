package tenant

import "context"

type Metadata struct {
	OrganizationID     string `json:"organization_id"`
	OrganizationSlug   string `json:"organization_slug"`
	StorageRevision    string `json:"storage_revision"`
	Status             string `json:"status"`
	AccessKeyID        string `json:"access_key_id"`
	SecretAccessKey    string `json:"secret_access_key"`
	BucketName         string `json:"bucket_name"`
	StorageEndpointURL string `json:"storage_endpoint_url"`
	RepositoryName     string `json:"repository_name,omitempty"`
	RepositoryID       string `json:"repository_id,omitempty"`
}

type contextKey struct{}

func WithMetadata(ctx context.Context, metadata Metadata) context.Context {
	return context.WithValue(ctx, contextKey{}, metadata)
}

func FromContext(ctx context.Context) (Metadata, bool) {
	metadata, ok := ctx.Value(contextKey{}).(Metadata)
	return metadata, ok
}
