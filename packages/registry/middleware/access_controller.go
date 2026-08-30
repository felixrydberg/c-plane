package middleware

import (
	"crypto"
	"encoding/base64"
	"fmt"
	"net/http"
	"os"
	"strings"

	"github.com/distribution/distribution/v3/registry/auth"
	registrytoken "github.com/distribution/distribution/v3/registry/auth/token"
	"github.com/go-jose/go-jose/v4"
	"github.com/sirupsen/logrus"
)

const registryTokenKeyID = "cplane-registry"

func init() {
	if err := auth.Register("cplane", auth.InitFunc(newAccessController)); err != nil {
		logrus.Errorf("failed to register cplane auth: %v", err)
	}
}

type accessController struct {
	realm   string
	service string
	issuer  string
	secret  []byte
}

var _ auth.AccessController = (*accessController)(nil)

func newAccessController(options map[string]any) (auth.AccessController, error) {
	realm, err := requiredStringOption(options, "realm")
	if err != nil {
		return nil, err
	}
	service, err := requiredStringOption(options, "service")
	if err != nil {
		return nil, err
	}
	issuer, err := requiredStringOption(options, "issuer")
	if err != nil {
		return nil, err
	}

	encoded := os.Getenv("REGISTRY_TOKEN_SECRET")
	if encoded == "" {
		return nil, fmt.Errorf("REGISTRY_TOKEN_SECRET is required")
	}
	secret, err := base64.RawURLEncoding.DecodeString(encoded)
	if err != nil || len(secret) < 32 {
		return nil, fmt.Errorf("REGISTRY_TOKEN_SECRET must be base64url-encoded and contain at least 32 bytes")
	}

	return &accessController{
		realm:   realm,
		service: service,
		issuer:  issuer,
		secret:  secret,
	}, nil
}

func requiredStringOption(options map[string]any, name string) (string, error) {
	value, ok := options[name].(string)
	if !ok || value == "" {
		return "", fmt.Errorf("cplane auth requires a valid option string: %q", name)
	}
	return value, nil
}

func (ac *accessController) Authorized(req *http.Request, accessRecords ...auth.Access) (*auth.Grant, error) {
	scheme, rawToken, ok := strings.Cut(req.Header.Get("Authorization"), " ")
	if !ok || rawToken == "" || !strings.EqualFold(scheme, "bearer") {
		return nil, &challenge{realm: ac.realm, service: ac.service, access: accessRecords}
	}

	parsed, err := registrytoken.NewToken(rawToken, []jose.SignatureAlgorithm{jose.HS256})
	if err != nil {
		return nil, &challenge{realm: ac.realm, service: ac.service, access: accessRecords, err: err}
	}

	claims, err := parsed.Verify(registrytoken.VerifyOptions{
		TrustedIssuers:    []string{ac.issuer},
		AcceptedAudiences: []string{ac.service},
		TrustedKeys:       map[string]crypto.PublicKey{registryTokenKeyID: ac.secret},
	})
	if err != nil {
		return nil, &challenge{realm: ac.realm, service: ac.service, access: accessRecords, err: err}
	}

	for _, access := range accessRecords {
		if !claimAllows(claims.Access, access) {
			return nil, &challenge{realm: ac.realm, service: ac.service, access: accessRecords, err: auth.ErrInvalidCredential}
		}
	}

	return &auth.Grant{
		User:      auth.UserInfo{Name: claims.Subject},
		Resources: resources(claims.Access),
	}, nil
}

func claimAllows(claims []*registrytoken.ResourceActions, required auth.Access) bool {
	for _, claim := range claims {
		if claim == nil || claim.Type != required.Type || claim.Name != required.Name {
			continue
		}
		for _, action := range claim.Actions {
			if action == "*" || action == required.Action {
				return true
			}
		}
	}
	return false
}

func resources(claims []*registrytoken.ResourceActions) []auth.Resource {
	seen := make(map[auth.Resource]struct{}, len(claims))
	result := make([]auth.Resource, 0, len(claims))
	for _, claim := range claims {
		if claim == nil {
			continue
		}
		resource := auth.Resource{Type: claim.Type, Class: claim.Class, Name: claim.Name}
		if _, ok := seen[resource]; ok {
			continue
		}
		seen[resource] = struct{}{}
		result = append(result, resource)
	}
	return result
}

type challenge struct {
	realm   string
	service string
	access  []auth.Access
	err     error
}

var _ auth.Challenge = challenge{}

func (c challenge) Error() string {
	if c.err == nil {
		return fmt.Sprintf("cplane authentication challenge for realm %q", c.realm)
	}
	return fmt.Sprintf("cplane authentication challenge for realm %q: %v", c.realm, c.err)
}

func (c challenge) SetHeaders(_ *http.Request, w http.ResponseWriter) {
	value := fmt.Sprintf("Bearer realm=%q,service=%q", c.realm, c.service)
	if len(c.access) > 0 {
		scopes := make([]string, 0, len(c.access))
		for _, access := range c.access {
			scopes = append(scopes, fmt.Sprintf("%s:%s:%s", access.Type, access.Name, access.Action))
		}
		value += fmt.Sprintf(",scope=%q", strings.Join(scopes, " "))
	}
	w.Header().Set("WWW-Authenticate", value)
}
