#!/bin/bash
# env check. nobara/fedora path.

RED='\033[0;31m'
NC='\033[0m'

check_dep() {
    if pkg-config --exists "$1"; then
        echo "OK: $1"
    else
        echo -e "${RED}MISSING: $1${NC}"
        EXIT_CODE=1
    fi
}

EXIT_CODE=0

echo "checking deps..."

# core gst
check_dep gstreamer-1.0
check_dep gstreamer-base-1.0
check_dep gstreamer-app-1.0
check_dep gstreamer-video-1.0

# plugins / webrtc
check_dep gstreamer-bad-video-1.0
check_dep gstreamer-webrtc-1.0
check_dep gstreamer-sdp-1.0

# gstreamer elements check
if ! gst-inspect-1.0 nice >/dev/null 2>&1; then
    echo -e "${RED}MISSING: gstreamer nice plugin (libnice)${NC}"
    EXIT_CODE=1
fi

# x11/xcb/input
check_dep xcb
check_dep xrandr
check_dep libxdo

# rust/cargo
if command -v cargo >/dev/null 2>&1; then
    echo "OK: cargo"
else
    echo -e "${RED}MISSING: cargo${NC}"
    EXIT_CODE=1
fi

if [ $EXIT_CODE -ne 0 ]; then
    echo -e "\n${RED}deps missing. fix before build.${NC}"
    echo "try: sudo dnf install gstreamer1-devel gstreamer1-plugins-base-devel gstreamer1-plugins-bad-free-devel libxcb-devel libXrandr-devel"
    exit 1
fi

echo "env ready."
