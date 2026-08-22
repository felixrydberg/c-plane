import { describe, expect, it } from "vitest";
import {
  API_KEY_SCOPE_VALUES,
  MEMBER_PERMISSION_SCOPE_VALUES,
  ORGANIZATION_MANAGEMENT_SCOPE_VALUES,
} from "@cplane/migrations/utils";
import {
  denyAddMember,
  denyAssignPermissions,
  denyCreateInvitation,
  denyDeleteOrganization,
  denyRemoveMember,
  denyRenameOrganization,
  denyRevokeInvitation,
  denyUpdateMemberRole,
  hasScope,
  isOwner,
} from "../server/utils/permissions";

const owner = (permissions: string[] = []) => ({ role: "owner", permissions });
const member = (permissions: string[] = []) => ({ role: "member", permissions });

describe("scope vocabulary parity", () => {
  it("member permissions are a superset of the API-key scope vocabulary", () => {
    for (const scope of API_KEY_SCOPE_VALUES) {
      expect(MEMBER_PERMISSION_SCOPE_VALUES).toContain(scope);
    }
  });

  it("API keys never carry org-management scopes", () => {
    for (const management of ORGANIZATION_MANAGEMENT_SCOPE_VALUES) {
      expect(API_KEY_SCOPE_VALUES).not.toContain(management);
    }
    for (const management of ORGANIZATION_MANAGEMENT_SCOPE_VALUES) {
      expect(MEMBER_PERMISSION_SCOPE_VALUES).toContain(management);
    }
  });
});

describe("member-specific permissions", () => {
  it("owners hold every scope implicitly", () => {
    for (const scope of MEMBER_PERMISSION_SCOPE_VALUES) {
      expect(hasScope(owner(), scope)).toBe(true);
    }
    expect(isOwner(owner())).toBe(true);
  });

  it("members only hold what was granted", () => {
    const alice = member(["container:read", "bucket:create"]);
    expect(hasScope(alice, "container:read")).toBe(true);
    expect(hasScope(alice, "bucket:create")).toBe(true);
    expect(hasScope(alice, "project:delete")).toBe(false);
    expect(hasScope(alice, "org:update")).toBe(false);
    expect(hasScope(member(), "container:read")).toBe(false);
  });
});

describe("organization deletion", () => {
  it("is owner-only", () => {
    expect(denyDeleteOrganization(owner())).toBeNull();
    expect(denyDeleteOrganization(member(["org:update", "api-key:manage"]))).toMatch(
      /only organization owners/i,
    );
  });
});

describe("rename organization", () => {
  it("requires org:update or ownership", () => {
    expect(denyRenameOrganization(owner())).toBeNull();
    expect(denyRenameOrganization(member(["org:update"]))).toBeNull();
    expect(denyRenameOrganization(member(["registry:delete"]))).not.toBeNull();
  });
});

describe("invitation permissions", () => {
  it("validates that only owner or member roles are accepted", () => {
    const inviter = member(["member:invite"]);
    expect(denyCreateInvitation(inviter, "admin")).toMatch(/owner or member/i);
    expect(denyCreateInvitation(inviter, "superuser")).toMatch(/owner or member/i);
    expect(denyCreateInvitation(inviter, undefined)).toMatch(/owner or member/i);
    expect(denyCreateInvitation(inviter, "member")).toBeNull();
  });

  it("requires member:invite to invite at all", () => {
    expect(denyCreateInvitation(member([]), "member")).toMatch(/member:invite/);
    expect(denyCreateInvitation(owner(), "member")).toBeNull();
  });

  it("never lets a non-owner create an owner invitation", () => {
    expect(denyCreateInvitation(member(["member:invite"]), "owner")).toMatch(
      /only owners can create owner invitations/i,
    );
    expect(denyCreateInvitation(owner(), "owner")).toBeNull();
  });

  it("revoking invitations requires member:invite", () => {
    expect(denyRevokeInvitation(member(["member:invite"]))).toBeNull();
    expect(denyRevokeInvitation(member())).not.toBeNull();
    expect(denyRevokeInvitation(owner())).toBeNull();
  });
});

describe("membership hierarchy and owner protection", () => {
  const targetOwner = { role: "owner" };
  const targetMember = { role: "member" };

  it("members with member:remove may remove plain members only", () => {
    const remover = member(["member:remove"]);
    expect(denyRemoveMember(remover, targetMember, { isSelf: false, ownerCount: 1 })).toBeNull();
    expect(
      denyRemoveMember(remover, targetOwner, { isSelf: false, ownerCount: 2 }),
    ).toMatch(/only owners can remove an owner/i);
  });

  it("members without member:remove cannot remove anyone", () => {
    expect(
      denyRemoveMember(member(["container:read"]), targetMember, { isSelf: false, ownerCount: 1 }),
    ).toMatch(/member:remove/);
  });

  it("protects the final owner from removal", () => {
    expect(
      denyRemoveMember(owner(), targetOwner, { isSelf: false, ownerCount: 1 }),
    ).toMatch(/last owner/i);
    expect(
      denyRemoveMember(owner(), targetOwner, { isSelf: false, ownerCount: 2 }),
    ).toBeNull();
  });

  it("blocks self-removal through the removal action", () => {
    expect(
      denyRemoveMember(owner(), targetOwner, { isSelf: true, ownerCount: 2 }),
    ).toMatch(/leave organization/i);
  });

  it("role changes stay owner-only with a final-owner guard", () => {
    expect(
      denyUpdateMemberRole(member(["org:update"]), "member", targetMember, {
        isSelf: false,
        ownerCount: 1,
      }),
    ).toMatch(/only owners can change member roles/i);

    expect(
      denyUpdateMemberRole(owner(), "admin", targetMember, { isSelf: false, ownerCount: 1 }),
    ).toMatch(/owner or member/i);

    expect(
      denyUpdateMemberRole(owner(), "member", targetOwner, { isSelf: true, ownerCount: 1 }),
    ).toMatch(/last owner/i);

    expect(
      denyUpdateMemberRole(owner(), "member", targetOwner, { isSelf: true, ownerCount: 2 }),
    ).toBeNull();
  });
});

describe("permission assignment", () => {
  it("is owner-only", () => {
    expect(denyAssignPermissions(owner(), targetMemberShape)).toBeNull();
    expect(denyAssignPermissions(member(["org:update"]), targetMemberShape)).toMatch(
      /only owners can manage member permissions/i,
    );
  });

  it("owners cannot attach permissions to other owners", () => {
    expect(denyAssignPermissions(owner(), targetOwnerShape)).toMatch(/members only/i);
  });
});

// denyAssignPermissions takes {role} shapes; keep literals local for readability.
const targetMemberShape = { role: "member" };
const targetOwnerShape = { role: "owner" };

describe("direct API bypass attempts (privileged endpoint manifest)", () => {
  // Every privileged UI server endpoint and the decision that guards it.
  // If a new privileged endpoint is added, register it here so it ships with its guard.
  const privilegedEndpoints: Array<{
    endpoint: string;
    guard: (subject: { role: string; permissions: string[] }) => string | null;
    bypassing: { role: string; permissions: string[] };
  }> = [
    {
      endpoint: "DELETE /api/organization/:id",
      guard: denyDeleteOrganization,
      bypassing: member(["*"]),
    },
    {
      endpoint: "PUT /api/organization/:id/name",
      guard: denyRenameOrganization,
      bypassing: member([]),
    },
    {
      endpoint: "POST /api/organization/:id/members",
      guard: denyAddMember,
      bypassing: member([]),
    },
    {
      endpoint: "DELETE /api/organization/:id/members/:member_id",
      guard: (s) =>
        denyRemoveMember(s, targetMemberShape, { isSelf: false, ownerCount: 2 }),
      bypassing: member([]),
    },
    {
      endpoint: "PATCH /api/organization/:id/members/:member_id",
      guard: (s) =>
        denyUpdateMemberRole(s, "member", targetMemberShape, { isSelf: false, ownerCount: 1 }),
      bypassing: member(["member:remove"]),
    },
    {
      endpoint: "PUT /api/organization/:id/members/:member_id/permissions",
      guard: (s) => denyAssignPermissions(s, targetMemberShape),
      bypassing: member(["member:invite", "member:remove"]),
    },
    {
      endpoint: "POST /api/organization/:id/invitations",
      guard: (s) => denyCreateInvitation(s, "member"),
      bypassing: member([]),
    },
    {
      endpoint: "DELETE /api/organization/:id/invitations/:invitation_id",
      guard: denyRevokeInvitation,
      bypassing: member([]),
    },
  ];

  it.each(privilegedEndpoints)(
    "$endpoint denies members lacking the required permission",
    ({ guard, bypassing }) => {
      expect(guard(bypassing)).not.toBeNull();
      expect(guard(owner())).toBeNull();
    }
  );

  it("no member scope subset unlocks owner-only endpoints", () => {
    // A member granted *every* grantable scope still cannot delete the org
    // or manage permissions — those are not scopes.
    const maximallyPrivilegedMember = member([...MEMBER_PERMISSION_SCOPE_VALUES]);
    expect(denyDeleteOrganization(maximallyPrivilegedMember)).not.toBeNull();
    expect(denyAssignPermissions(maximallyPrivilegedMember, targetMemberShape)).not.toBeNull();
    expect(
      denyRemoveMember(maximallyPrivilegedMember, targetOwnerShape, {
        isSelf: false,
        ownerCount: 9,
      }),
    ).not.toBeNull();
  });
});
