#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfirmAction {
    ClearQueue,
    ClearHistory,
    ClearCache,
    DeletePlaylist { id: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaletteMode {
    CommandSelection,
    CreatePlaylistInput,
    DeletePlaylistSelection,
    AddToPlaylistSelection,
    SeekInput,
    SpotifyImportInput,
    ThemeSelection,
    Confirmation(ConfirmAction),
    ContextActions,
}

#[derive(Debug, Clone)]
pub struct CommandOption {
    pub name: &'static str,
    pub description: &'static str,
}

pub const AVAILABLE_COMMANDS: &[CommandOption] = &[
    CommandOption {
        name: "theme: set",
        description: "Choose a predefined color theme",
    },
    CommandOption {
        name: "playlist: create",
        description: "Create a new local playlist",
    },
    CommandOption {
        name: "playlist: delete",
        description: "Delete an existing local playlist",
    },
    CommandOption {
        name: "playlist: import spotify",
        description: "Import tracks from a public Spotify playlist",
    },
    CommandOption {
        name: "queue: clear",
        description: "Clear the playback queue",
    },
    CommandOption {
        name: "history: clear",
        description: "Clear the recently played track history",
    },
    CommandOption {
        name: "seek",
        description: "Seek to a specific time (e.g. 30, 1:30)",
    },
    CommandOption {
        name: "cache: clear",
        description: "Clear the downloaded local audio cache",
    },
    CommandOption {
        name: "view: toggle lyrics",
        description: "Toggle the lyrics overlay panel",
    },
    CommandOption {
        name: "view: toggle queue",
        description: "Toggle the side queue panel",
    },
    CommandOption {
        name: "view: toggle history",
        description: "Toggle the recently played tracks panel",
    },
    CommandOption {
        name: "view: toggle help",
        description: "Toggle the help overlay screen",
    },
    CommandOption {
        name: "layout: toggle resize",
        description: "Toggle panel resize mode",
    },
    CommandOption {
        name: "playback: play/pause",
        description: "Play or pause current playback",
    },
    CommandOption {
        name: "playback: stop",
        description: "Stop current audio and clear player state",
    },
    CommandOption {
        name: "playback: next",
        description: "Play the next track in the queue",
    },
    CommandOption {
        name: "playback: previous",
        description: "Play the previous track in the queue",
    },
    CommandOption {
        name: "playback: toggle shuffle",
        description: "Toggle queue shuffle mode",
    },
    CommandOption {
        name: "playback: toggle repeat",
        description: "Cycle repeat mode (None -> All -> One)",
    },
    CommandOption {
        name: "playback: toggle auto play",
        description: "Toggle song radio auto-play",
    },
    CommandOption {
        name: "playback: repeat: none",
        description: "Disable repeat mode",
    },
    CommandOption {
        name: "playback: repeat: all",
        description: "Repeat all tracks in the queue",
    },
    CommandOption {
        name: "playback: repeat: one",
        description: "Repeat the current track",
    },
];

