# SPDX-License-Identifier: GPL-3.0-or-later
# shellcheck disable=SC2207
# bash completion for auto-tlp
_auto_tlp() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    case "$prev" in
        -p|--profile) COMPREPLY=( $(compgen -W "balanced performance battery" -- "$cur") ); return ;;
        -o|--output)  COMPREPLY=( $(compgen -f -- "$cur") ); return ;;
        --only|--skip) COMPREPLY=( $(compgen -W "general cpu platform graphics storage pcie radios usb audio kernel battery" -- "$cur") ); return ;;
        --format)      COMPREPLY=( $(compgen -W "conf nix json" -- "$cur") ); return ;;
        --against)     COMPREPLY=( $(compgen -f -- "$cur") ); return ;;
        --sysroot)     COMPREPLY=( $(compgen -d -- "$cur") ); return ;;
        --start-thresh|--stop-thresh|--tlp-version) return ;;
    esac
    COMPREPLY=( $(compgen -W "-p --profile --start-thresh --stop-thresh -o --output --install -r --report -v --verbose --only --skip --format --uninstall --apply --diff --against --sysroot --terse --tlp-version --demo -h --help -V --version" -- "$cur") )
}
complete -F _auto_tlp auto-tlp
