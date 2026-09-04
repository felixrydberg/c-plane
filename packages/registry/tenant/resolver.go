package tenant

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strings"
)

func Resolve(ctx context.Context, client *http.Client, controlPlaneURL, serviceToken, organizationID, repositoryName, repositoryID string) (Metadata, error) {
	endpoint := strings.TrimRight(controlPlaneURL, "/") + "/internal/organizations/" + url.PathEscape(organizationID) + "/registry"
	if repositoryName != "" || repositoryID != "" {
		query := url.Values{"repository_name": {repositoryName}, "repository_id": {repositoryID}}
		endpoint += "?" + query.Encode()
	}
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, endpoint, nil)
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
	if repositoryName != "" && (metadata.RepositoryName != repositoryName || metadata.RepositoryID != repositoryID) {
		return Metadata{}, fmt.Errorf("resolver returned mismatched repository metadata")
	}
	return metadata, nil
}
