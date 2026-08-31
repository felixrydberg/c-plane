package middleware

import (
	"encoding/base64"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/distribution/distribution/v3/registry/auth"
	"github.com/golang-jwt/jwt/v5"
)

func TestAccessControllerAuthorizesHS256Token(t *testing.T) {
	secret := []byte("01234567890123456789012345678901")
	t.Setenv("REGISTRY_TOKEN_SECRET", base64.RawURLEncoding.EncodeToString(secret))

	controller, err := newAccessController(map[string]any{
		"realm":   "http://localhost:8080/api/registry/token",
		"service": "localhost:5000",
		"issuer":  "cplane-registry",
	})
	if err != nil {
		t.Fatal(err)
	}

	rawToken := signedToken(t, secret, "localhost:5000", "acme/api", "pull")
	request := httptest.NewRequest(http.MethodGet, "/v2/acme/api/manifests/latest", nil)
	request.Header.Set("Authorization", "Bearer "+rawToken)

	grant, err := controller.Authorized(request, auth.Access{
		Resource: auth.Resource{Type: "repository", Name: "acme/api"},
		Action:   "pull",
	})
	if err != nil {
		t.Fatal(err)
	}
	if grant.User.Name != "registry-user" {
		t.Fatalf("grant user = %q, want registry-user", grant.User.Name)
	}
	if len(grant.Resources) != 1 || grant.Resources[0].Name != "acme/api" {
		t.Fatalf("grant resources = %#v", grant.Resources)
	}
}

func TestAccessControllerRejectsUnauthorizedAction(t *testing.T) {
	secret := []byte("01234567890123456789012345678901")
	t.Setenv("REGISTRY_TOKEN_SECRET", base64.RawURLEncoding.EncodeToString(secret))

	controller, err := newAccessController(map[string]any{
		"realm":   "http://localhost:8080/api/registry/token",
		"service": "localhost:5000",
		"issuer":  "cplane-registry",
	})
	if err != nil {
		t.Fatal(err)
	}

	rawToken := signedToken(t, secret, "localhost:5000", "acme/api", "pull")
	request := httptest.NewRequest(http.MethodPut, "/v2/acme/api/manifests/latest", nil)
	request.Header.Set("Authorization", "Bearer "+rawToken)

	_, err = controller.Authorized(request, auth.Access{
		Resource: auth.Resource{Type: "repository", Name: "acme/api"},
		Action:   "push",
	})
	if _, ok := err.(auth.Challenge); !ok {
		t.Fatalf("error = %T %v, want auth challenge", err, err)
	}
}

func signedToken(t *testing.T, secret []byte, audience, repository, action string) string {
	t.Helper()
	now := time.Now()
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims{
		Access: []accessClaim{{Type: "repository", Name: repository, Actions: []string{action}}},
		RegisteredClaims: jwt.RegisteredClaims{
			Issuer:    "cplane-registry",
			Subject:   "registry-user",
			Audience:  jwt.ClaimStrings{audience},
			ExpiresAt: jwt.NewNumericDate(now.Add(time.Minute)),
			IssuedAt:  jwt.NewNumericDate(now),
		},
	})
	token.Header["kid"] = registryTokenKeyID
	rawToken, err := token.SignedString(secret)
	if err != nil {
		t.Fatal(err)
	}
	return rawToken
}
