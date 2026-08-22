// Pure authorization decisions for organization management.
// Every function returns null when the action is allowed, or a reason string.
// Kept free of Nuxt/h3 imports so it can be unit-tested directly.

export const ROLES = ["owner", "member"] as const;
export type Role = (typeof ROLES)[number];

export const INVITATION_ROLES = ROLES;

export type Subject = {
  role: string;
  permissions: readonly string[];
};

export const isOwner = (subject: Subject) => subject.role === "owner";

export const hasScope = (subject: Subject, scope: string) =>
  isOwner(subject) || subject.permissions.includes(scope);

const forbidden = (reason: string) => reason;

export function denyDeleteOrganization(actor: Subject): string | null {
  if (!isOwner(actor)) {
    return forbidden("Only organization owners can delete the organization");
  }
  return null;
}

export function denyRenameOrganization(actor: Subject): string | null {
  if (!hasScope(actor, "org:update")) {
    return forbidden("Missing required permission: org:update");
  }
  return null;
}

export function denyCreateInvitation(
  actor: Subject,
  invitedRole: unknown,
): string | null {
  if (!INVITATION_ROLES.includes(invitedRole as Role)) {
    return forbidden("Role must be either owner or member");
  }
  if (!hasScope(actor, "member:invite")) {
    return forbidden("Missing required permission: member:invite");
  }
  if (invitedRole === "owner" && !isOwner(actor)) {
    return forbidden("Only owners can create owner invitations");
  }
  return null;
}

export function denyRevokeInvitation(actor: Subject): string | null {
  if (!hasScope(actor, "member:invite")) {
    return forbidden("Missing required permission: member:invite");
  }
  return null;
}

export function denyAddMember(actor: Subject): string | null {
  if (!hasScope(actor, "member:invite")) {
    return forbidden("Missing required permission: member:invite");
  }
  return null;
}

export function denyRemoveMember(
  actor: Subject,
  target: { role: string },
  options: { isSelf: boolean; ownerCount: number },
): string | null {
  if (options.isSelf) {
    return forbidden("Use the leave organization action to remove yourself");
  }
  if (target.role === "owner") {
    if (!isOwner(actor)) {
      return forbidden("Only owners can remove an owner");
    }
    if (options.ownerCount <= 1) {
      return forbidden("Cannot remove the last owner of the organization");
    }
    return null;
  }
  if (!hasScope(actor, "member:remove")) {
    return forbidden("Missing required permission: member:remove");
  }
  return null;
}

export function denyUpdateMemberRole(
  actor: Subject,
  nextRole: unknown,
  target: { role: string },
  options: { isSelf: boolean; ownerCount: number },
): string | null {
  if (!isOwner(actor)) {
    return forbidden("Only owners can change member roles");
  }
  if (!ROLES.includes(nextRole as Role)) {
    return forbidden("Role must be either owner or member");
  }
  if (options.isSelf && target.role === "owner" && nextRole !== "owner" && options.ownerCount <= 1) {
    return forbidden("Cannot demote the last owner of the organization");
  }
  return null;
}

export function denyAssignPermissions(
  actor: Subject,
  target: { role: string },
): string | null {
  if (!isOwner(actor)) {
    return forbidden("Only owners can manage member permissions");
  }
  if (target.role === "owner") {
    return forbidden("Owners have full access; permissions apply to members only");
  }
  return null;
}
