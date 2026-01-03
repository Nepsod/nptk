//! Menu command IDs - hybrid system with enum for standard commands and u32 for custom extensions

use std::fmt;

/// Unified menu command identifier system.
/// Standard commands use typed enums for type safety, while custom/extension commands
/// use u32 values for flexibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuCommand {
    // File menu commands
    FileNew,
    FileOpen,
    FileSave,
    FileSaveAs,
    FileClose,
    FileDelete,
    FileRename,
    FileProperties,
    FileLink, // Create shortcut
    FileExit,

    // Edit menu commands
    EditUndo,
    EditRedo,
    EditCut,
    EditCopy,
    EditPaste,
    EditPasteLink, // Paste shortcut
    EditSelectAll,
    EditDeselectAll,

    // View menu commands
    ViewIcon,
    ViewSmallIcon,
    ViewList,
    ViewDetails,
    ViewOptions,
    ArrangeAuto,
    ArrangeGrid,

    // Custom/extension commands (0x1000+ range for extensions)
    Custom(u32),
}

impl MenuCommand {
    /// First ID for custom commands (extensions should use 0x1000+)
    pub const CUSTOM_FIRST: u32 = 0x1000;

    /// Convert to u32 for command routing
    pub fn to_u32(&self) -> u32 {
        match self {
            MenuCommand::FileNew => 0x0001,
            MenuCommand::FileOpen => 0x0002,
            MenuCommand::FileSave => 0x0003,
            MenuCommand::FileSaveAs => 0x0004,
            MenuCommand::FileClose => 0x0005,
            MenuCommand::FileDelete => 0x0006,
            MenuCommand::FileRename => 0x0007,
            MenuCommand::FileProperties => 0x0008,
            MenuCommand::FileLink => 0x0009,
            MenuCommand::FileExit => 0x000A,
            MenuCommand::EditUndo => 0x0100,
            MenuCommand::EditRedo => 0x0101,
            MenuCommand::EditCut => 0x0102,
            MenuCommand::EditCopy => 0x0103,
            MenuCommand::EditPaste => 0x0104,
            MenuCommand::EditPasteLink => 0x0105,
            MenuCommand::EditSelectAll => 0x0106,
            MenuCommand::EditDeselectAll => 0x0107,
            MenuCommand::ViewIcon => 0x0200,
            MenuCommand::ViewSmallIcon => 0x0201,
            MenuCommand::ViewList => 0x0202,
            MenuCommand::ViewDetails => 0x0203,
            MenuCommand::ViewOptions => 0x0204,
            MenuCommand::ArrangeAuto => 0x0205,
            MenuCommand::ArrangeGrid => 0x0206,
            MenuCommand::Custom(id) => *id,
        }
    }

    /// Try to convert from u32 back to MenuCommand
    pub fn from_u32(id: u32) -> Self {
        match id {
            0x0001 => MenuCommand::FileNew,
            0x0002 => MenuCommand::FileOpen,
            0x0003 => MenuCommand::FileSave,
            0x0004 => MenuCommand::FileSaveAs,
            0x0005 => MenuCommand::FileClose,
            0x0006 => MenuCommand::FileDelete,
            0x0007 => MenuCommand::FileRename,
            0x0008 => MenuCommand::FileProperties,
            0x0009 => MenuCommand::FileLink,
            0x000A => MenuCommand::FileExit,
            0x0100 => MenuCommand::EditUndo,
            0x0101 => MenuCommand::EditRedo,
            0x0102 => MenuCommand::EditCut,
            0x0103 => MenuCommand::EditCopy,
            0x0104 => MenuCommand::EditPaste,
            0x0105 => MenuCommand::EditPasteLink,
            0x0106 => MenuCommand::EditSelectAll,
            0x0107 => MenuCommand::EditDeselectAll,
            0x0200 => MenuCommand::ViewIcon,
            0x0201 => MenuCommand::ViewSmallIcon,
            0x0202 => MenuCommand::ViewList,
            0x0203 => MenuCommand::ViewDetails,
            0x0204 => MenuCommand::ViewOptions,
            0x0205 => MenuCommand::ArrangeAuto,
            0x0206 => MenuCommand::ArrangeGrid,
            id if id >= Self::CUSTOM_FIRST => MenuCommand::Custom(id),
            _ => MenuCommand::Custom(id), // Unknown command, treat as custom
        }
    }
}

impl fmt::Display for MenuCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MenuCommand::FileNew => write!(f, "FileNew"),
            MenuCommand::FileOpen => write!(f, "FileOpen"),
            MenuCommand::FileSave => write!(f, "FileSave"),
            MenuCommand::FileSaveAs => write!(f, "FileSaveAs"),
            MenuCommand::FileClose => write!(f, "FileClose"),
            MenuCommand::FileDelete => write!(f, "FileDelete"),
            MenuCommand::FileRename => write!(f, "FileRename"),
            MenuCommand::FileProperties => write!(f, "FileProperties"),
            MenuCommand::FileLink => write!(f, "FileLink"),
            MenuCommand::FileExit => write!(f, "FileExit"),
            MenuCommand::EditUndo => write!(f, "EditUndo"),
            MenuCommand::EditRedo => write!(f, "EditRedo"),
            MenuCommand::EditCut => write!(f, "EditCut"),
            MenuCommand::EditCopy => write!(f, "EditCopy"),
            MenuCommand::EditPaste => write!(f, "EditPaste"),
            MenuCommand::EditPasteLink => write!(f, "EditPasteLink"),
            MenuCommand::EditSelectAll => write!(f, "EditSelectAll"),
            MenuCommand::EditDeselectAll => write!(f, "EditDeselectAll"),
            MenuCommand::ViewIcon => write!(f, "ViewIcon"),
            MenuCommand::ViewSmallIcon => write!(f, "ViewSmallIcon"),
            MenuCommand::ViewList => write!(f, "ViewList"),
            MenuCommand::ViewDetails => write!(f, "ViewDetails"),
            MenuCommand::ViewOptions => write!(f, "ViewOptions"),
            MenuCommand::ArrangeAuto => write!(f, "ArrangeAuto"),
            MenuCommand::ArrangeGrid => write!(f, "ArrangeGrid"),
            MenuCommand::Custom(id) => write!(f, "Custom({:#x})", id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_roundtrip() {
        let commands = vec![
            MenuCommand::FileNew,
            MenuCommand::EditCut,
            MenuCommand::ViewIcon,
            MenuCommand::Custom(0x1000),
            MenuCommand::Custom(0x2000),
        ];

        for cmd in commands {
            let id = cmd.to_u32();
            let restored = MenuCommand::from_u32(id);
            assert_eq!(cmd, restored, "Failed roundtrip for {:?}", cmd);
        }
    }
}
