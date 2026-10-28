# ELYTerminal: restore ZDOTDIR, load the user's .zshenv, report folder and title, mark prompts and commands.
if [[ -n "${ELY_ZSH_ZDOTDIR+set}" ]]; then
  ZDOTDIR=$ELY_ZSH_ZDOTDIR
  builtin unset ELY_ZSH_ZDOTDIR
else
  builtin unset ZDOTDIR
fi
[[ -r "${ZDOTDIR:-$HOME}/.zshenv" ]] && builtin source "${ZDOTDIR:-$HOME}/.zshenv"
if [[ -o interactive ]]; then
  _ely_ran=
  _ely_first=1
  _ely_precmd() {
    builtin local ret=$? LC_ALL=C char url= dir=$PWD
    if [[ -n $_ely_ran ]]; then
      builtin printf '\e]133;D;%s\a' $ret
      _ely_ran=
    fi
    if [[ -n ${ELY_WSL-} ]]; then
      # Under WSL the folder goes as Windows names it: a drive's, or the distribution's share.
      dir=${"$(wslpath -w $PWD 2>/dev/null)"//\\//}
      case $dir in //*) ;; ?:*) dir=/$dir ;; *) dir= ;; esac
    fi
    for char in ${(s::)dir}; do
      case $char in
        [[:cntrl:]]|%) builtin printf -v char '%%%02X' "'$char" ;;
      esac
      url+=$char
    done
    [[ -n $dir ]] && builtin printf '\e]7;file://%s\a' "$url"
    builtin printf '\e]2;%s\a' "${${${PWD:t}:-/}//[[:cntrl:]]/}"
    # Themes set PS1 in later hooks: the marks go on after them, at the first prompt through sched.
    precmd_functions=(${precmd_functions:#_ely_mark} _ely_mark)
    if (( _ely_first )); then
      _ely_first=0
      [[ -o prompt_percent ]] || builtin print -ru2 'ELYTerminal: zsh marks no prompt while prompt_percent is off (setopt prompt_percent)'
      builtin zmodload zsh/sched && builtin sched +0 _ely_mark $ret
    fi
    builtin return ret
  }
  _ely_mark() {
    if [[ -o prompt_percent && $PS1 != *$'\e]133;A'* ]]; then
      PS1=$'%{\e]133;A;aid=elyterminal\a%}'$PS1$'%{\e]133;B\a%}'
    fi
    # sched's command leaves its status for the prompt.
    builtin return ${1:-0}
  }
  _ely_preexec() {
    _ely_ran=1
    builtin printf '\e]133;C\a\e]2;%s\a' "${1//[[:cntrl:]]/}"
  }
  builtin autoload -Uz add-zsh-hook
  add-zsh-hook precmd _ely_precmd
  add-zsh-hook precmd _ely_mark
  add-zsh-hook preexec _ely_preexec
fi
