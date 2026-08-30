package tenant

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
)

func Resolve(ctx context.Context, client *http.Client, controlPlaneURL, serviceToken, organizationID string) (Metadata, error) {
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, strings.TrimRight(controlPlaneURL, "/")+"/internal/organizations/"+url.PathEscape(organizationID)+"/registry", nil)
	if err != nil {
		return Metadata{}, err
	}
	request.Header.Set("x-cplane-token", serviceToken)
	response, err := client.Do(request)
	if err != nil {
		return Metadata{}, err
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return Metadata{}, fmt.Errorf("resolver returned %s", response.Status)
	}
	var metadata Metadata
	if err := json.NewDecoder(response.Body).Decode(&metadata); err != nil {
		return Metadata{}, err
	}
	if metadata.OrganizationID != organizationID || metadata.BucketName != "registry" {
		return Metadata{}, fmt.Errorf("resolver returned mismatched metadata")
	}
	return metadata, nil
}
