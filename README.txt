PocketJS frontend for BaseOS (RG34XX)

This payload installs the approved PocketJS launcher as BaseOS's generic
frontend. Extract it into the SD card root; frontend-session invokes
System/launch_frontend.sh through /bin/sh.

Runtime: System/pocketjs/bin/pocket-linux-kms
Launcher: System/pocketjs/apps/boot.pocket
Logs: /tmp/pocketjs-frontend.log (RAM)

The script checks package admission before launch and probes /dev/dri/card0,
then card1. If admission fails, it does not start the package. Repeated runtime
exits back off; after three consecutive failures the delay is 30 seconds.

BaseOS has no guest filesystem bridge to enumerate /mnt/sdcard/games, and the
current PocketJS Linux KMS runtime does not support child package switching.
Library scanning and launching child packages are unavailable in this stage.
SSH and ADB are enabled by BaseOS defaults. Configure hostname/mdns in
baseos.conf at the SD root.
