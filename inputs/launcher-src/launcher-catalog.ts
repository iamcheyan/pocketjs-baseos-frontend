export interface LauncherApp {
  id: string;
  name: string;
  shortName: string;
  description: string;
  kind: "native" | "terminal";
  category: "home" | "media" | "games" | "system";
  icon: string;
  tag: string;
}

export const LAUNCHER_APPS: readonly LauncherApp[] = [
  {
    id: "terminal",
    name: "Terminal",
    shortName: "Terminal",
    description: "Interactive command shell",
    kind: "terminal",
    category: "home",
    icon: "\uf120",
    tag: "SHELL"
  },
  {
    id: "disk",
    name: "File Manager",
    shortName: "Files",
    description: "Browse files and storage volumes",
    kind: "native",
    category: "home",
    icon: "\uf07b",
    tag: "NATIVE"
  },
  {
    id: "xmplayer",
    name: "XMPlayer",
    shortName: "XMPlayer",
    description: "Music and video player",
    kind: "native",
    category: "media",
    icon: "\uf001",
    tag: "NATIVE"
  },
  {
    id: "monitor",
    name: "System Monitor",
    shortName: "Monitor",
    description: "CPU, memory and process monitor",
    kind: "terminal",
    category: "home",
    icon: "\uf0e4",
    tag: "TOP"
  },
  {
    id: "file-manager-text",
    name: "Midnight Commander",
    shortName: "MC",
    description: "Classic dual-pane visual file manager",
    kind: "terminal",
    category: "home",
    icon: "\uf115",
    tag: "MC"
  },
  {
    id: "notes",
    name: "Text Editor",
    shortName: "Notes",
    description: "Edit persistent notes with nano",
    kind: "terminal",
    category: "home",
    icon: "\uf044",
    tag: "NANO"
  },
  {
    id: "audio-mixer",
    name: "Audio Mixer",
    shortName: "Mixer",
    description: "ALSA device and volume controls",
    kind: "terminal",
    category: "media",
    icon: "\uf028",
    tag: "ALSA"
  },
  {
    id: "gamepad",
    name: "Gamepad Test",
    shortName: "Gamepad",
    description: "Test buttons and directional controls",
    kind: "terminal",
    category: "games",
    icon: "\uf11b",
    tag: "TEST"
  },
  {
    id: "clean-mac-files",
    name: "Clean macOS Metadata",
    shortName: "Cleaner",
    description: "Remove ._* files from the ROM directory",
    kind: "terminal",
    category: "games",
    icon: "\uf1f8",
    tag: "TOOL"
  },
  {
    id: "cloud-backup",
    name: "Cloud Backup",
    shortName: "Backup",
    description: "PJOS Linux rclone backup",
    kind: "terminal",
    category: "games",
    icon: "\uf0ee",
    tag: "SYNC"
  },
  {
    id: "cloud-restore",
    name: "Cloud Restore",
    shortName: "Restore",
    description: "PJOS Linux rclone restore, with confirmation",
    kind: "terminal",
    category: "games",
    icon: "\uf0ed",
    tag: "SYNC"
  },
  {
    id: "system-info",
    name: "System Information",
    shortName: "SysInfo",
    description: "PJOS Linux hardware and software information",
    kind: "terminal",
    category: "system",
    icon: "\uf2db",
    tag: "INFO"
  },
  {
    id: "processes",
    name: "Process Manager",
    shortName: "Processes",
    description: "Process list and resource usage",
    kind: "terminal",
    category: "system",
    icon: "\uf085",
    tag: "PS"
  },
  {
    id: "storage-info",
    name: "Storage Information",
    shortName: "Storage",
    description: "Filesystem usage and mounted volumes",
    kind: "terminal",
    category: "system",
    icon: "\uf0a0",
    tag: "DISK"
  },
  {
    id: "network-info",
    name: "Network Information",
    shortName: "Network",
    description: "Network devices and active connections",
    kind: "terminal",
    category: "system",
    icon: "\uf012",
    tag: "NET"
  },
  {
    id: "usb-info",
    name: "USB Devices",
    shortName: "USB",
    description: "Connected USB devices",
    kind: "terminal",
    category: "system",
    icon: "\uf287",
    tag: "USB"
  },
  {
    id: "input-info",
    name: "Input Devices",
    shortName: "Input",
    description: "Input devices recognized by libinput",
    kind: "terminal",
    category: "system",
    icon: "\uf11c",
    tag: "INPUT"
  },
  {
    id: "kernel-log",
    name: "Kernel Logs",
    shortName: "Dmesg",
    description: "Kernel and driver messages",
    kind: "terminal",
    category: "system",
    icon: "\uf188",
    tag: "DMESG"
  },
  {
    id: "system-log",
    name: "System Logs",
    shortName: "Logs",
    description: "Recent logs from the current boot",
    kind: "terminal",
    category: "system",
    icon: "\uf02d",
    tag: "LOGS"
  }
] as const;

export function getAppsForScreen(screen: number): LauncherApp[] {
  switch (screen) {
    case 0: // Screen 0: Home primary shortcuts
      return LAUNCHER_APPS.filter((app) =>
        ["terminal", "disk", "xmplayer", "monitor", "file-manager-text", "notes", "audio-mixer"].includes(app.id)
      );
    case 1: // Screen 1: Media apps
      return LAUNCHER_APPS.filter((app) =>
        ["xmplayer", "audio-mixer", "disk"].includes(app.id)
      );
    case 2: // Screen 2: Games and ROM utilities
      return LAUNCHER_APPS.filter((app) =>
        ["gamepad", "clean-mac-files", "cloud-backup", "cloud-restore"].includes(app.id)
      );
    case 4: // Screen 4: System tools
      return LAUNCHER_APPS.filter((app) =>
        ["system-info", "monitor", "processes", "storage-info", "network-info", "usb-info", "input-info", "kernel-log", "system-log"].includes(app.id)
      );
    default:
      return [];
  }
}
