package middleware

import (
	"context"
	"crypto/subtle"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"

	registrydriver "github.com/cplane/cplane/registry/driver"
	"github.com/cplane/cplane/registry/garbagecollection"
	"github.com/cplane/cplane/registry/telemetry"
	"github.com/cplane/cplane/registry/tenant"
	"github.com/distribution/distribution/v3/configuration"
	storagedriver "github.com/distribution/distribution/v3/registry/storage/driver"
	"github.com/golang-jwt/jwt/v5"
	"github.com/google/uuid"
	"github.com/sirupsen/logrus"
)

const (
	garbageCollectionPathPrefix = "/internal/organizations/"
	garbageCollectionPathSuffix = "/garbage-collection"
)

type accessClaim struct {
	Type         string   `json:"type"`
	Name         string   `json:"name"`
	Actions      []string `json:"actions"`
	RepositoryID string   `json:"repository_id"`
}

type claims struct {
	OrganizationID string        `json:"organization_id"`
	Access         []accessClaim `json:"access"`
	jwt.RegisteredClaims
}

type Handler struct {
	next              http.Handler
	secret            []byte
	issuer            string
	audience          string
	controlPlaneURL   string
	serviceToken      string
	http              *http.Client
	garbageCollect    func(context.Context, string, string) (garbagecollection.Report, error)
	organizationLocks sync.Map
}

func New(_ *configuration.Configuration, next http.Handler) http.Handler {
	secret, err := base64.RawURLEncoding.DecodeString(os.Getenv("REGISTRY_TOKEN_SECRET"))
	if err != nil || len(secret) < 32 {
		panic("REGISTRY_TOKEN_SECRET must be base64url-encoded and contain at least 32 bytes")
	}
	controlPlaneURL := getenv("CPLANE_API_URL", "http://api:8080")
	serviceToken := os.Getenv("CPLANE_SERVICE_TOKEN")
	client := &http.Client{Timeout: 5 * time.Second}
	return &Handler{
		next:            next,
		secret:          secret,
		issuer:          getenv("REGISTRY_TOKEN_ISSUER", "cplane-registry"),
		audience:        getenv("REGISTRY_HOST", "localhost:5000"),
		controlPlaneURL: controlPlaneURL,
		serviceToken:    serviceToken,
		http:            client,
		garbageCollect: func(ctx context.Context, organizationID string, jobID string) (garbagecollection.Report, error) {
			return garbagecollection.Run(ctx, client, controlPlaneURL, serviceToken, organizationID, jobID)
		},
	}
}

func (h *Handler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	if strings.HasPrefix(r.URL.Path, garbageCollectionPathPrefix) {
		if strings.Contains(strings.TrimPrefix(r.URL.Path, garbageCollectionPathPrefix), "/repositories/") {
			h.serveRepositoryDelete(w, r)
			return
		}
		h.serveGarbageCollection(w, r)
		return
	}
	if r.URL.Path == "/v2/_catalog" || strings.HasPrefix(r.URL.Path, "/v2/_catalog?") {
		writeOCIError(w, http.StatusNotFound, "NAME_UNKNOWN", "catalog access is disabled")
		return
	}
	raw := strings.TrimPrefix(r.Header.Get("Authorization"), "Bearer ")
	if raw == "" || raw == r.Header.Get("Authorization") {
		h.next.ServeHTTP(w, r)
		return
	}
	parsed := &claims{}
	token, err := jwt.ParseWithClaims(raw, parsed, func(token *jwt.Token) (any, error) {
		if token.Method.Alg() != jwt.SigningMethodHS256.Alg() {
			return nil, fmt.Errorf("unexpected signing algorithm")
		}
		return h.secret, nil
	}, jwt.WithAudience(h.audience), jwt.WithIssuer(h.issuer), jwt.WithExpirationRequired())
	if err != nil || !token.Valid || parsed.OrganizationID == "" {
		telemetry.AuthenticationFailures.Inc()
		logrus.WithField("event", "registry_authentication_failed").Warn("registry request rejected")
		writeOCIError(w, http.StatusUnauthorized, "UNAUTHORIZED", "invalid registry token")
		return
	}
	organizationLock := h.organizationLock(parsed.OrganizationID)
	organizationLock.RLock()
	defer organizationLock.RUnlock()
	repositoryName, repositoryID := repositoryForRequest(r.URL.Path, parsed.Access)
	if strings.HasPrefix(r.URL.Path, "/v2/") && r.URL.Path != "/v2/" && repositoryName == "" {
		telemetry.AuthenticationFailures.Inc()
		writeOCIError(w, http.StatusUnauthorized, "UNAUTHORIZED", "repository scope is not bound to this request")
		return
	}
	resolverStarted := time.Now()
	metadata, err := tenant.Resolve(r.Context(), h.http, h.controlPlaneURL, h.serviceToken, parsed.OrganizationID, repositoryName, repositoryID)
	telemetry.ResolverLatency.Observe(time.Since(resolverStarted).Seconds())
	if err != nil {
		telemetry.ResolverRequests.WithLabelValues("error").Inc()
		logrus.WithError(err).WithField("event", "registry_resolver_failed").Error("managed registry resolution failed")
		if repositoryName != "" {
			writeOCIError(w, http.StatusUnauthorized, "UNAUTHORIZED", "repository authorization is no longer valid")
			return
		}
		writeOCIError(w, http.StatusServiceUnavailable, "UNAVAILABLE", "managed registry metadata is unavailable")
		return
	}
	telemetry.ResolverRequests.WithLabelValues("success").Inc()
	prefix := metadata.OrganizationSlug + "/"
	for _, access := range parsed.Access {
		if access.Type == "repository" && !strings.HasPrefix(access.Name, prefix) {
			telemetry.AuthenticationFailures.Inc()
			logrus.WithField("event", "registry_scope_rejected").Warn("registry request rejected")
			writeOCIError(w, http.StatusUnauthorized, "UNAUTHORIZED", "repository scope belongs to another organization")
			return
		}
	}
	if metadata.Status != "active" {
		telemetry.WriteRejections.Inc()
		logrus.WithFields(logrus.Fields{"event": "registry_access_rejected", "organization_id": metadata.OrganizationID}).Warn("managed registry is unavailable")
		writeOCIError(w, http.StatusServiceUnavailable, "UNAVAILABLE", "registry is unavailable during maintenance")
		return
	}
	h.next.ServeHTTP(w, r.WithContext(tenant.WithMetadata(r.Context(), metadata)))
}

func (h *Handler) serveGarbageCollection(w http.ResponseWriter, r *http.Request) {
	organizationID, found := strings.CutPrefix(r.URL.Path, garbageCollectionPathPrefix)
	organizationID, suffixFound := strings.CutSuffix(organizationID, garbageCollectionPathSuffix)
	if !found || !suffixFound || organizationID == "" || strings.Contains(organizationID, "/") {
		http.NotFound(w, r)
		return
	}
	providedToken := r.Header.Get("x-cplane-token")
	if h.serviceToken == "" || subtle.ConstantTimeCompare([]byte(providedToken), []byte(h.serviceToken)) != 1 {
		logrus.WithField("organization_id", organizationID).Warn("unauthorized managed Registry garbage-collection request")
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	if r.Method != http.MethodPost {
		w.Header().Set("Allow", http.MethodPost)
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}
	fields := logrus.Fields{"organization_id": organizationID}
	jobID := r.Header.Get("x-cplane-job-id")
	if jobID != "" {
		fields["job_id"] = jobID
	}
	logger := logrus.WithFields(fields)
	started := time.Now()
	logger.Info("managed Registry garbage-collection request started")
	drainStarted := time.Now()
	logger.Info("waiting for active managed Registry requests to drain")
	organizationLock := h.organizationLock(organizationID)
	organizationLock.Lock()
	defer organizationLock.Unlock()
	logger.WithField("duration_ms", time.Since(drainStarted).Milliseconds()).Info("exclusive managed Registry access acquired for garbage collection")
	report, err := h.garbageCollect(r.Context(), organizationID, jobID)
	if err != nil {
		logger.WithError(err).WithField("duration_ms", time.Since(started).Milliseconds()).Error("managed Registry garbage-collection request failed")
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	logger.WithFields(logrus.Fields{
		"bytes_after":     report.BytesAfter,
		"bytes_before":    report.BytesBefore,
		"bytes_reclaimed": report.BytesBefore - report.BytesAfter,
		"duration_ms":     time.Since(started).Milliseconds(),
	}).Info("managed Registry garbage-collection request completed")
	w.Header().Set("Content-Type", "application/json")
	if err := json.NewEncoder(w).Encode(report); err != nil {
		logrus.WithError(err).WithField("organization_id", organizationID).Error("managed registry garbage-collection response failed")
	}
}

func (h *Handler) serveRepositoryDelete(w http.ResponseWriter, r *http.Request) {
	path := strings.TrimPrefix(r.URL.Path, garbageCollectionPathPrefix)
	parts := strings.Split(path, "/")
	if len(parts) != 3 || parts[0] == "" || parts[1] != "repositories" {
		http.NotFound(w, r)
		return
	}
	organizationID, repositoryID := parts[0], parts[2]
	if _, err := uuid.Parse(organizationID); err != nil {
		http.NotFound(w, r)
		return
	}
	if _, err := uuid.Parse(repositoryID); err != nil {
		http.NotFound(w, r)
		return
	}
	providedToken := r.Header.Get("x-cplane-token")
	if h.serviceToken == "" || subtle.ConstantTimeCompare([]byte(providedToken), []byte(h.serviceToken)) != 1 {
		http.Error(w, "unauthorized", http.StatusUnauthorized)
		return
	}
	if r.Method != http.MethodDelete {
		w.Header().Set("Allow", http.MethodDelete)
		http.Error(w, "method not allowed", http.StatusMethodNotAllowed)
		return
	}

	organizationLock := h.organizationLock(organizationID)
	organizationLock.RLock()
	defer organizationLock.RUnlock()
	metadata, err := tenant.Resolve(r.Context(), h.http, h.controlPlaneURL, h.serviceToken, organizationID, "", "")
	if err != nil {
		http.Error(w, err.Error(), http.StatusBadGateway)
		return
	}
	storage, err := registrydriver.NewForTenant(r.Context(), metadata)
	if err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	err = storage.Delete(r.Context(), "/docker/registry/v2/repositories/"+repositoryID)
	if !repositoryDeleteSucceeded(err) {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func repositoryDeleteSucceeded(err error) bool {
	if err == nil {
		return true
	}
	var missing storagedriver.PathNotFoundError
	return errors.As(err, &missing)
}

func repositoryForRequest(path string, accesses []accessClaim) (string, string) {
	var name, id string
	for _, access := range accesses {
		prefix := "/v2/" + access.Name + "/"
		if access.Type == "repository" && access.RepositoryID != "" && strings.HasPrefix(path, prefix) && len(access.Name) > len(name) {
			name, id = access.Name, access.RepositoryID
		}
	}
	return name, id
}

func (h *Handler) organizationLock(organizationID string) *sync.RWMutex {
	lock, _ := h.organizationLocks.LoadOrStore(organizationID, &sync.RWMutex{})
	return lock.(*sync.RWMutex)
}

func writeOCIError(w http.ResponseWriter, status int, code, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Retry-After", "30")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]any{"errors": []map[string]string{{"code": code, "message": message}}})
}

func getenv(name, fallback string) string {
	if value := os.Getenv(name); value != "" {
		return value
	}
	return fallback
}
