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
  _ely_pipe=
  _ely_precmd() {
    builtin local ret=$? pipe="${pipestatus[*]}" LC_ALL=C char url= dir=$PWD
    # The user's options may not bend the script.
    builtin setopt local_options unset no_ksh_arrays no_sh_word_split no_err_return no_err_exit
    _ely_pipe=$pipe
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
      # The line editor is on by the first prompt, not in .zshenv.
      if [[ -o zle ]] && ! { builtin zmodload zsh/zle && builtin autoload -Uz +X add-zle-hook-widget 2>/dev/null && add-zle-hook-widget line-init _ely_line_init; }; then
        builtin print -ru2 'ELYTerminal: zsh could not hook its line editor; a theme moving its hook last goes unmarked'
      fi
    fi
    builtin return ret
  }
  _ely_mark() {
    builtin setopt local_options unset no_ksh_arrays no_sh_word_split no_err_return no_err_exit
    builtin local a=$'\e]133;A;aid=elyterminal' p=$'\e]133;P;k=s' r=';redraw=1'
    if [[ -o prompt_percent && $PS1 != *$'\e]133;A'* ]]; then
      PS1=$'%{'$a$'\a%}'$PS1$'%{\e]133;B\a%}'
    fi
    # Only the line editor redraws a prompt on a resize, and it may go.
    PS1=${PS1//"$a$r"/$a}
    [[ -o zle ]] && PS1=${PS1//"$a"/$a$r}
    # An unset PS2 stays unset.
    if (( ${+PS2} )); then
      if [[ -o prompt_percent && $PS2 != *$'\e]133;P'* ]]; then
        PS2=$'%{'$p$'\a%}'$PS2$'%{\e]133;B\a%}'
      fi
      PS2=${PS2//"$p$r"/$p}
      [[ -o zle ]] && PS2=${PS2//"$p"/$p$r}
    fi
    # sched's command leaves its status for the prompt.
    builtin return ${1:-0}
  }
  # A theme's later hook set PS1 again: mark it here.
  _ely_line_init() {
    builtin setopt local_options unset no_ksh_arrays no_sh_word_split no_err_return no_err_exit
    if [[ -o prompt_percent && $PS1 != *$'\e]133;A'* ]]; then
      _ely_mark
      builtin local one line=
      builtin local -a statuses
      statuses=(${=_ely_pipe:-0})
      # reset-prompt expands at once: give it the command's statuses.
      if (( $#statuses == 1 )); then
        line="_ely_status $statuses"
      else
        # Functions in a pipeline report statuses past 128 unreliably.
        for one in $statuses; do line+="( builtin exit $one ) | "; done
        line=${line% | }
      fi
      builtin eval "$line; zle .reset-prompt"
    fi
  }
  _ely_status() { builtin return $1 }
  _ely_preexec() {
    _ely_ran=1
    builtin printf '\e]133;C\a\e]2;%s\a' "${1//[[:cntrl:]]/}"
  }
  builtin autoload -Uz add-zsh-hook
  add-zsh-hook precmd _ely_precmd
  add-zsh-hook precmd _ely_mark
  add-zsh-hook preexec _ely_preexec
fi
