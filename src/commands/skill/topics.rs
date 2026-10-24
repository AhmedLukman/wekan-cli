pub(super) struct Topic {
    pub name: &'static str,
    pub description: &'static str,
    pub content: &'static str,
}

// These maintained documents are compiled into the executable, including for
// cargo install. Keep Cargo.toml's package includes in sync when adding topics.
pub(super) const TOPICS: &[Topic] = &[
    Topic {
        name: "admin",
        description: "Administrator domains, organizations, and teams",
        content: include_str!("../../../docs/admin.md"),
    },
    Topic {
        name: "admin-settings",
        description: "Global settings and attachment storage settings",
        content: include_str!("../../../docs/admin-settings.md"),
    },
    Topic {
        name: "agent-contract",
        description: "Shared envelopes, streams, errors, confirmations, and exit statuses",
        content: include_str!("../../../docs/agent-contract.md"),
    },
    Topic {
        name: "api",
        description: "Raw API requests, authentication, bodies, and response handling",
        content: include_str!("../../../docs/api.md"),
    },
    Topic {
        name: "attachments",
        description: "Attachment inspection, uploads, downloads, and transfers",
        content: include_str!("../../../docs/attachments.md"),
    },
    Topic {
        name: "auth",
        description: "Login, registration, logout, status, and secret input",
        content: include_str!("../../../docs/auth.md"),
    },
    Topic {
        name: "boards",
        description: "Board lifecycle, settings, domains, and attachment links",
        content: include_str!("../../../docs/boards.md"),
    },
    Topic {
        name: "bulk-cards-and-labels",
        description: "Bulk card operations and board/card labels",
        content: include_str!("../../../docs/bulk-cards-and-labels.md"),
    },
    Topic {
        name: "cards",
        description: "Card discovery, creation, updates, archive, and deletion",
        content: include_str!("../../../docs/cards.md"),
    },
    Topic {
        name: "checklists",
        description: "Checklists and checklist items",
        content: include_str!("../../../docs/checklists.md"),
    },
    Topic {
        name: "comments",
        description: "Card comment reads and mutations",
        content: include_str!("../../../docs/comments.md"),
    },
    Topic {
        name: "config",
        description: "Named profiles, server selection, and configuration precedence",
        content: include_str!("../../../docs/config.md"),
    },
    Topic {
        name: "copy-and-move",
        description: "Native resource copies, moves, and pinned server limitations",
        content: include_str!("../../../docs/copy-and-move.md"),
    },
    Topic {
        name: "core",
        description: "Start here: agent workflows, discovery, and essential conventions",
        content: include_str!("../../../docs/agent-guide.md"),
    },
    Topic {
        name: "custom-fields",
        description: "Custom-field definitions, dropdown items, and card values",
        content: include_str!("../../../docs/custom-fields.md"),
    },
    Topic {
        name: "dependencies",
        description: "Card dependency lines and their stable results",
        content: include_str!("../../../docs/dependencies.md"),
    },
    Topic {
        name: "exports",
        description: "Board, card, and attachment exports and output delivery",
        content: include_str!("../../../docs/exports.md"),
    },
    Topic {
        name: "imports",
        description: "Board, Trello, ZIP, and calendar imports and partial failures",
        content: include_str!("../../../docs/imports.md"),
    },
    Topic {
        name: "integrations",
        description: "Board webhook subscriptions and integration activity selection",
        content: include_str!("../../../docs/integrations.md"),
    },
    Topic {
        name: "lists",
        description: "Board lists, positions, and WIP limits",
        content: include_str!("../../../docs/lists.md"),
    },
    Topic {
        name: "memberships",
        description: "Board/card members, assignees, and permissions",
        content: include_str!("../../../docs/memberships.md"),
    },
    Topic {
        name: "rules",
        description: "Board automation rules, triggers, actions, and buttons",
        content: include_str!("../../../docs/rules.md"),
    },
    Topic {
        name: "swimlanes",
        description: "Swimlane discovery and lifecycle operations",
        content: include_str!("../../../docs/swimlanes.md"),
    },
    Topic {
        name: "system",
        description: "Public schema-upgrade status",
        content: include_str!("../../../docs/system.md"),
    },
    Topic {
        name: "updates",
        description: "CLI update notifications, installation methods, and automation",
        content: include_str!("../../../docs/updates.md"),
    },
    Topic {
        name: "users",
        description: "User inspection, administration, ownership, and tokens",
        content: include_str!("../../../docs/users.md"),
    },
];
