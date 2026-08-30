package middleware

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/cplane/cplane/registry/garbagecollection"
	"github.com/cplane/cplane/registry/tenant"
	"github.com/golang-jwt/jwt/v5"
)

func TestHandlerInjectsResolvedTenant(t *testing.T) {
	resolver := resolverServer(t, "active")
	defer resolver.Close()
	called := false
	h := testHandler(t, resolver.URL, http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		metadata, ok := tenant.FromContext(r.Context())
		if !ok || metadata.OrganizationID != "org-1" || metadata.BucketName != "registry" {
			t.Fatalf("unexpected metadata: %#v, %v", metadata, ok)
		}
		called = true
		w.WriteHeader(http.StatusNoContent)
	}))

	response := request(t, h, http.MethodGet, "/v2/acme/api/manifests/latest", token(t, "org-1", "acme/api"))
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

	response := request(t, h, http.MethodGet, "/v2/other/api/manifests/latest", token(t, "org-1", "other/api"))
	assertOCIError(t, response, http.StatusUnauthorized, "UNAUTHORIZED")
}

func TestHandlerGatesAllAccessDuringOrganizationGC(t *testing.T) {
	resolver := resolverServer(t, "maintenance")
	defer resolver.Close()
	h := testHandler(t, resolver.URL, http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusNoContent)
	}))

	mutation := request(t, h, http.MethodPut, "/v2/acme/api/manifests/latest", token(t, "org-1", "acme/api"))
	assertOCIError(t, mutation, http.StatusServiceUnavailable, "UNAVAILABLE")
	pull := request(t, h, http.MethodGet, "/v2/acme/api/manifests/latest", token(t, "org-1", "acme/api"))
	assertOCIError(t, pull, http.StatusServiceUnavailable, "UNAVAILABLE")
}

func TestHandlerDisablesCatalog(t *testing.T) {
	h := &Handler{next: http.NotFoundHandler()}
	response := request(t, h, http.MethodGet, "/v2/_catalog", "")
	assertOCIError(t, response, http.StatusNotFound, "NAME_UNKNOWN")
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
	request := httptest.NewRequest(http.MethodPost, "/internal/organizations/org-1/garbage-collection", nil)
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
	request := httptest.NewRequest(http.MethodPost, "/internal/organizations/org-1/garbage-collection", nil)
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
		_ = json.NewEncoder(w).Encode(tenant.Metadata{
			OrganizationID:     "org-1",
			OrganizationSlug:   "acme",
			StorageRevision:    "revision-1",
			AccessKeyID:        "access-key",
			SecretAccessKey:    "secret-key",
			BucketName:         "registry",
			StorageEndpointURL: "http://storage:8081",
			Status:             status,
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

func token(t *testing.T, organizationID, repository string) string {
	t.Helper()
	now := time.Now()
	encoded, err := jwt.NewWithClaims(jwt.SigningMethodHS256, claims{
		OrganizationID: organizationID,
		Access:         []accessClaim{{Type: "repository", Name: repository, Actions: []string{"pull"}}},
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
	request := httptest.NewRequest(method, path, nil)
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
