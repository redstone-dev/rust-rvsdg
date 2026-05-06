functions -c fish_prompt orig_prompt
set -g inside_nix_shell
function fish_prompt
    echo -ne "\x1b[31mnix shell \x1b[0m:: "(orig_prompt)
end
