#!/bin/bash

DEVICE_LABEL="RPI-RP2"
MOUNT_POINT="/tmp/rp2040-$$"
          
function usage() {  
    echo "Usage: $0 -d <central|peripheral>" 1>&2;
    exit 1;
}

while getopts ":d:" arg; do 
    case "${arg}" in
        d)
            target=${OPTARG}
            if [[ "$target" != "central" && "$target" != "peripheral" ]]; then 
                usage
            fi
            ;;
        *)
            usage
            ;;
    esac
done

if [[ -z "$target" ]]; then
    usage
fi

uf2_file="./target/thumbv6m-none-eabi/release/${target}.uf2"

sudo -v || exit 1

echo "Waiting for RP2040 in BOOTSEL mode..."

while true; do
    DEVICE=$(lsblk -o NAME,LABEL -rn | awk -v label="$DEVICE_LABEL" '$2 == label {print "/dev/" $1}')
    [[ -n "$DEVICE" ]] && break
    sleep 0.5
done

echo "Device found at $DEVICE"

sudo mkdir -p "$MOUNT_POINT"

echo "Mounting..."
sudo mount "$DEVICE" "$MOUNT_POINT"

echo "Copying $uf2_file..."
sudo cp "$uf2_file" "$MOUNT_POINT/"

sudo umount "$MOUNT_POINT"
sudo rmdir "$MOUNT_POINT"

echo "Done!"