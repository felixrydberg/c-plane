INSERT INTO "user" ("id", "name", "email", "email_verified", "image", "created_at", "updated_at", "role", "banned", "ban_reason", "ban_expires", "two_factor_enabled")
VALUES ('3336daf6-36b8-4855-a1d0-168b711dc9cf', 'felix', 'felixryd@proton.me', false, null, '2026-02-12 18:09:57.685', '2026-02-12 18:09:57.685', 'user', false, null, null, false);

INSERT INTO "account" ("id", "account_id", "provider_id", "user_id", "access_token", "refresh_token", "id_token", "access_token_expires_at", "refresh_token_expires_at", "scope", "password", "created_at", "updated_at")
VALUES ('a16ca4e9-dd1a-4531-80cd-4c3a5285f377', '3336daf6-36b8-4855-a1d0-168b711dc9cf', 'credential', '3336daf6-36b8-4855-a1d0-168b711dc9cf', null, null, null, null, null, null, 'ca4aa4374c2148a316e8dbc3d1ed1c61:0c25dc481c21054da3bfcdee6acd92eaf103a607c3076c98f12f48701b3bac33be55345f91e6a327b432b75952ea224f624d477cb427f0a118eda70c9461919c', '2026-02-12 18:09:57.698', '2026-02-12 18:09:57.698');

INSERT INTO "organization" ("id", "name", "email", "slug", "logo", "created_at", "polar_customer_id")
VALUES ('07fdaa2a-c89b-4fcf-886b-c35f079df0d5', 'felix''s Organization', 'felixryd@proton.me', 'felixs-organization', null, '2026-02-12 18:47:08.638927', 'e61d6259-038b-49f8-b549-aa55f99c679d');

INSERT INTO "active_organization" ("user_id", "organization_id")
VALUES ('3336daf6-36b8-4855-a1d0-168b711dc9cf', '07fdaa2a-c89b-4fcf-886b-c35f079df0d5');

INSERT INTO "organization_member" ("id", "organization_id", "user_id", "role", "created_at")
VALUES ('92a0278c-b232-475f-a944-44a89c959464', '07fdaa2a-c89b-4fcf-886b-c35f079df0d5', '3336daf6-36b8-4855-a1d0-168b711dc9cf', 'owner', '2026-02-12 18:47:08.64');

INSERT INTO "studio_user" ("id", "name", "email", "email_verified", "image", "created_at", "updated_at", "role", "banned", "ban_reason", "ban_expires", "two_factor_enabled")
VALUES ('3336daf6-36b8-4855-a1d0-168b711dc9cf', 'felix', 'felixryd@proton.me', false, null, '2026-02-12 18:09:57.685', '2026-02-12 18:09:57.685', 'user', false, null, null, false);

INSERT INTO "studio_account" ("id", "account_id", "provider_id", "user_id", "access_token", "refresh_token", "id_token", "access_token_expires_at", "refresh_token_expires_at", "scope", "password", "created_at", "updated_at")
VALUES ('a16ca4e9-dd1a-4531-80cd-4c3a5285f377', '3336daf6-36b8-4855-a1d0-168b711dc9cf', 'credential', '3336daf6-36b8-4855-a1d0-168b711dc9cf', null, null, null, null, null, null, 'ca4aa4374c2148a316e8dbc3d1ed1c61:0c25dc481c21054da3bfcdee6acd92eaf103a607c3076c98f12f48701b3bac33be55345f91e6a327b432b75952ea224f624d477cb427f0a118eda70c9461919c', '2026-02-12 18:09:57.698', '2026-02-12 18:09:57.698');
