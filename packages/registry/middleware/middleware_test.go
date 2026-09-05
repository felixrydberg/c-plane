package middleware

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/cplane/cplane/registry/garbagecollection"
	"github.com/cplane/cplane/registry/tenant"
	storagedriver "github.com/distribution/distribution/v3/registry/storage/driver"
	"github.com/golang-jwt/jwt/v5"
)

const (
	projectID    = "11111111-1111-4111-8111-111111111111"
	repositoryID = "22222222-2222-4222-8222-222222222222"
)

func TestHandlerInjectsResolvedTenant(t *testing.T) {
	resolver := resolverServer(t, "active")
	defer resolver.Close()
	called := false
	h := testHandler(t, resolver.URL, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		metadata, ok := tenant.FromContext(r.Context())
		if !ok || metadata.OrganizationID != "org-1" || metadata.BucketName != "registry" || metadata.RepositoryID != repositoryID {
			t.Fatalf("unexpected metadata: %#v, %v", metadata, ok)
		}
		called = true
		w.WriteHeader(http.StatusNoContent)
	}))

	repository := "acme/" + projectID + "/api"
	response := request(t, h, http.MethodGet, "/v2/"+repository+"/manifests/latest", token(t, "org-1", repository, repositoryID))
	if response.Code != http.StatusNoContent || !called {
		t.Fatalf("status=%d called=%v", response.Code, called)
	}
}

func TestHandlerRejectsCrossOrganizationScope(t *testing.T) {
	resolver := resolverServer(t, "active")
	defer resolver.Close()
	h := testHandler(t, resolver.URL, http.HandlerFunc(func(http.ResponseWriter, *http.Request) {
		t.Fatal("downstream handler must not be called")
	}))

	repository := "other/" + projectID + "/api"
	response := request(t, h, http.MethodGet, "/v2/"+repository+"/manifests/latest", token(t, "org-1", repository, repositoryID))
	assertOCIError(t, response, http.StatusUnauthorized, "UNAUTHORIZED")
}

func TestHandlerBlocksWritesDuringOrganizationGC(t *testing.T) {
	resolver := resolverServer(t, "maintenance")
	defer resolver.Close()
	called := false
	h := testHandler(t, resolver.URL, http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		called = true
		w.WriteHeader(http.StatusNoContent)
	}))

	repository := "acme/" + projectID + "/api"
	mutation := request(t, h, http.MethodPut, "/v2/"+repository+"/manifests/latest", token(t, "org-1", repository, repositoryID))
	assertOCIError(t, mutation, http.StatusServiceUnavailable, "UNAVAILABLE")
	deletion := request(t, h, http.MethodDelete, "/v2/"+repository+"/manifests/latest", token(t, "org-1", repository, repositoryID))
	assertOCIError(t, deletion, http.StatusServiceUnavailable, "UNAVAILABLE")
	pull := request(t, h, http.MethodGet, "/v2/"+repository+"/manifests/latest", token(t, "org-1", repository, repositoryID))
	if pull.Code != http.StatusNoContent || !called {
		t.Fatalf("reads must stay available during maintenance: status=%d called=%v", pull.Code, called)
	}
	tags := request(t, h, http.MethodGet, "/v2/"+repository+"/tags/list", token(t, "org-1", repository, repositoryID))
	if tags.Code != http.StatusNoContent {
		t.Fatalf("tag listing must stay available during maintenance: status=%d", tags.Code)
	}
}

func TestHandlerDisablesCatalog(t *testing.T) {
	h := &Handler{next: http.NotFoundHandler()}
	response := request(t, h, http.MethodGet, "/v2/_catalog", "")
	assertOCIError(t, response, http.StatusNotFound, "NAME_UNKNOWN")
}

func TestMissingRepositoryStorageIsAnIdempotentDelete(t *testing.T) {
	if !repositoryDeleteSucceeded(storagedriver.PathNotFoundError{Path: "/missing", DriverName: "test"}) {
		t.Fatal("missing repository must be treated as successfully deleted")
	}
	if repositoryDeleteSucceeded(errors.New("storage unavailable")) {
		t.Fatal("other storage failures must be retried")
	}
}

func TestHandlerRejectsRepositoryTokenAfterUUIDChanges(t *testing.T) {
	resolver := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Query().Get("repository_id") != "33333333-3333-4333-8333-333333333333" {
			http.NotFound(w, r)
			return
		}
	}))
	defer resolver.Close()
	h := testHandler(t, resolver.URL, http.HandlerFunc(func(http.ResponseWriter, *http.Request) {
		t.Fatal("downstream handler must not be called")
	}))
	repository := "acme/" + projectID + "/api"
	response := request(t, h, http.MethodGet, "/v2/"+repository+"/manifests/latest", token(t, "org-1", repository, repositoryID))
	assertOCIError(t, response, http.StatusUnauthorized, "UNAUTHORIZED")
}

func TestHandlerRunsInternalGarbageCollection(t *testing.T) {
	called := false
	h := &Handler{
		next:         http.NotFoundHandler(),
		serviceToken: "service-token",
		garbageCollect: func(_ context.Context, organizationID string, jobID string) (garbagecollection.Report, error) {
			if organizationID != "org-1" || jobID != "job-1" {
				t.Fatalf("organization=%q job=%q", organizationID, jobID)
			}
			called = true
			return garbagecollection.Report{BytesBefore: 100, BytesAfter: 25}, nil
		},
	}
	request := httptest.NewRequestWithContext(t.Context(), http.MethodPost, "/internal/organizations/org-1/garbage-collection", nil)
	request.Header.Set("x-cplane-token", "service-token")
	request.Header.Set("x-cplane-job-id", "job-1")
	response := httptest.NewRecorder()
	h.ServeHTTP(response, request)

	if response.Code != http.StatusOK || !called {
		t.Fatalf("status=%d called=%v", response.Code, called)
	}
	var report garbagecollection.Report
	if err := json.Unmarshal(response.Body.Bytes(), &report); err != nil {
		t.Fatal(err)
	}
	if report.BytesBefore != 100 || report.BytesAfter != 25 {
		t.Fatalf("unexpected report: %#v", report)
	}
}

func TestHandlerRejectsUnauthenticatedInternalGarbageCollection(t *testing.T) {
	h := &Handler{
		next:         http.NotFoundHandler(),
		serviceToken: "service-token",
		garbageCollect: func(context.Context, string, string) (garbagecollection.Report, error) {
			t.Fatal("garbage collection must not run")
			return garbagecollection.Report{}, nil
		},
	}
	request := httptest.NewRequestWithContext(t.Context(), http.MethodPost, "/internal/organizations/org-1/garbage-collection", nil)
	response := httptest.NewRecorder()
	h.ServeHTTP(response, request)

	if response.Code != http.StatusUnauthorized {
		t.Fatalf("status=%d", response.Code)
	}
}

func resolverServer(t *testing.T, status string) *httptest.Server {
	t.Helper()
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("x-cplane-token") != "service-token" {
			t.Fatal("missing service token")
		}
		repositoryName := r.URL.Query().Get("repository_name")
		resolvedRepositoryID := r.URL.Query().Get("repository_id")
		_ = json.NewEncoder(w).Encode(tenant.Metadata{
			OrganizationID:     "org-1",
			OrganizationSlug:   "acme",
			StorageRevision:    "revision-1",
			AccessKeyID:        "access-key",
			SecretAccessKey:    "secret-key",
			BucketName:         "registry",
			StorageEndpointURL: "http://storage:8081",
			Status:             status,
			RepositoryName:     repositoryName,
			RepositoryID:       resolvedRepositoryID,
		})
	}))
}

func testHandler(t *testing.T, resolverURL string, next http.Handler) *Handler {
	t.Helper()
	return &Handler{
		next:            next,
		secret:          []byte("01234567890123456789012345678901"),
		issuer:          "cplane-registry",
		audience:        "registry.example.com",
		controlPlaneURL: resolverURL,
		serviceToken:    "service-token",
		http:            resolverServerClient(),
	}
}

func resolverServerClient() *http.Client { return &http.Client{Timeout: time.Second} }

func token(t *testing.T, organizationID, repository, resolvedRepositoryID string) string {
	t.Helper()
	now := time.Now()
	encoded, err := jwt.NewWithClaims(jwt.SigningMethodHS256, claims{
		OrganizationID: organizationID,
		Access:         []accessClaim{{Type: "repository", Name: repository, Actions: []string{"pull"}, RepositoryID: resolvedRepositoryID}},
		RegisteredClaims: jwt.RegisteredClaims{
			Issuer:    "cplane-registry",
			Audience:  jwt.ClaimStrings{"registry.example.com"},
			ExpiresAt: jwt.NewNumericDate(now.Add(time.Minute)),
			IssuedAt:  jwt.NewNumericDate(now),
		},
	}).SignedString([]byte("01234567890123456789012345678901"))
	if err != nil {
		t.Fatal(err)
	}
	return encoded
}

func request(t *testing.T, handler http.Handler, method, path, token string) *httptest.ResponseRecorder {
	t.Helper()
	request := httptest.NewRequestWithContext(t.Context(), method, path, nil)
	if token != "" {
		request.Header.Set("Authorization", "Bearer "+token)
	}
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	return response
}

func assertOCIError(t *testing.T, response *httptest.ResponseRecorder, status int, code string) {
	t.Helper()
	if response.Code != status {
		t.Fatalf("status=%d, want %d", response.Code, status)
	}
	var body struct {
		Errors []struct {
			Code string `json:"code"`
		} `json:"errors"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &body); err != nil {
		t.Fatal(err)
	}
	if len(body.Errors) != 1 || body.Errors[0].Code != code {
		t.Fatalf("unexpected OCI error: %#v", body)
	}
}
