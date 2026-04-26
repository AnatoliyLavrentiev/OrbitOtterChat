use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Role {
    Owner,
    Admin,
    Member,
}

pub fn can_create_channel(role: Role) -> bool {
    matches!(role, Role::Admin | Role::Owner)
}

pub fn can_update_channel(role: Role) -> bool {
    matches!(role, Role::Admin | Role::Owner)
}

pub fn can_delete_channel(role: Role) -> bool {
    matches!(role, Role::Admin | Role::Owner)
}

pub fn can_delete_message(role: Role, is_owner_of_message: bool) -> bool {
    match role {
        Role::Owner | Role::Admin => true,
        Role::Member => is_owner_of_message,
    }
}

pub fn can_manage_roles(role: Role) -> bool {
    matches!(role, Role::Owner)
}

pub fn owner_cannot_leave_server(role: Role) -> bool {
    matches!(role, Role::Owner)
}

pub fn can_leave_server(role: Role) -> bool {
    !owner_cannot_leave_server(role)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_channel() {
        assert!(!can_create_channel(Role::Member));
        assert!(can_create_channel(Role::Admin));
        assert!(can_create_channel(Role::Owner));
    }

    #[test]
    fn test_can_delete_message() {
        assert!(can_delete_message(Role::Member, true));
        assert!(!can_delete_message(Role::Member, false));

        assert!(can_delete_message(Role::Admin, true));
        assert!(can_delete_message(Role::Admin, false));

        assert!(can_delete_message(Role::Owner, true));
        assert!(can_delete_message(Role::Owner, false));
    }

    #[test]
    fn test_owner_leave_rules() {
        assert!(owner_cannot_leave_server(Role::Owner));
        assert!(!owner_cannot_leave_server(Role::Admin));
        assert!(!owner_cannot_leave_server(Role::Member));

        assert!(!can_leave_server(Role::Owner));
        assert!(can_leave_server(Role::Admin));
        assert!(can_leave_server(Role::Member));
    }

    #[test]
    fn test_manage_roles() {
        assert!(!can_manage_roles(Role::Member));
        assert!(!can_manage_roles(Role::Admin));
        assert!(can_manage_roles(Role::Owner));
    }

    #[test]
    fn test_channel_crud() {
        assert!(!can_update_channel(Role::Member));
        assert!(can_update_channel(Role::Admin));
        assert!(can_update_channel(Role::Owner));

        assert!(!can_delete_channel(Role::Member));
        assert!(can_delete_channel(Role::Admin));
        assert!(can_delete_channel(Role::Owner));
    }
}
