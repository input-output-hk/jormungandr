#! /bin/sh

COLOR_NC='\e[0m' # No Color
COLOR_WHITE='\e[1;37m'
COLOR_BLACK='\e[0;30m'
COLOR_BLUE='\e[0;34m'
COLOR_LIGHT_BLUE='\e[1;34m'
COLOR_GREEN='\e[0;32m'
COLOR_LIGHT_GREEN='\e[1;32m'
COLOR_CYAN='\e[0;36m'
COLOR_LIGHT_CYAN='\e[1;36m'
COLOR_RED='\e[0;31m'
COLOR_LIGHT_RED='\e[1;31m'
COLOR_PURPLE='\e[0;35m'
COLOR_LIGHT_PURPLE='\e[1;35m'
COLOR_BROWN='\e[0;33m'
COLOR_YELLOW='\e[1;33m'
COLOR_GRAY='\e[0;30m'
COLOR_LIGHT_GRAY='\e[0;37m'

display() {
    printf "${*}"
}

success() {
    display "${COLOR_LIGHT_GREEN}${*}${COLOR_NC}"
}

info() {
    display "${COLOR_LIGHT_CYAN}${*}${COLOR_NC}"
}

warn() {
    display "${COLOR_YELLOW}${*}${COLOR_NC}"
}

error() {
    display "${COLOR_LIGHT_RED}${*}${COLOR_NC}"
}

newline() {
    display "\n"
}

title() {
    newline
    info "  ${*}"
    newline
    newline
}

die() {
    error ${*}
    newline
    exit 1
}

jcli=$(pwd)/target/debug/jcli
if [ ! -x ${jcli} ]; then
    jcli='cargo run --bin jcli --quiet --'
fi

jormungandr=$(pwd)/target/debug/jormungandr
if [ ! -x ${jormungandr} ]; then
    jormungandr='cargo run --bin jormungandr --'
fi

warn "path to jcli:        ${jcli}\n"
warn "path to jormungandr: ${jormungandr}\n"
