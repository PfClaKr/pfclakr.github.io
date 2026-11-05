+++
title = "setting_linux : configurer une nouvelle machine Linux, un vimrc et un prompt bash"
description = "La liste de ce que je configure sur une machine Linux neuve, et le dépôt setting_linux que j'emporte : un install.sh qui applique tout d'un coup, chaque réglage du vimrc et sa raison d'être, le fonctionnement du prompt bash sans oh-my-zsh, et le thème pour qui utilise zsh. Le tout vérifié en l'exécutant."
tags = ["linux", "vim", "bash", "zsh", "shell", "dotfiles"]
+++

Les fichiers que je copie en premier sur chaque nouvelle machine Linux sont rassemblés dans le dépôt
[setting_linux](https://github.com/PfClaKr/setting_linux). L'essentiel tient en un vimrc et un prompt bash, appliqués par
un seul script d'installation. Comme il ne faut que bash, le même environnement fonctionne sur les serveurs et les
conteneurs, et un thème zsh au même aspect est fourni pour les machines qui utilisent oh-my-zsh.

```text
setting_linux/
├── install.sh          applique vimrc + prompt d'un coup
├── vimrc               → ~/.vimrc
├── ychun.bash-theme    le prompt bash
├── extras.bash         réglages de l'historique, alias
├── ychun.zsh-theme     thème pour oh-my-zsh (installé avec --zsh)
└── .zshrc
```

Cet article parcourt la liste pour une machine neuve, le script d'installation, puis les fichiers du dépôt
(vimrc, prompt bash, `extras.bash`, thème zsh). Tout a été vérifié sous Ubuntu 24.04 avec Vim 9.1, bash 5.2 et zsh 5.9.

## Sur une nouvelle machine Linux

C'est pour Debian et Ubuntu. Poste de travail ou serveur, c'est presque pareil ; les étapes propres aux serveurs
sont signalées.

**1. Mettre les paquets à jour et installer les outils de base.**

```bash
sudo apt update && sudo apt full-upgrade -y
sudo apt install -y git vim curl wget build-essential htop tmux tree unzip ripgrep fd-find jq
```

Sur la famille Debian, l'exécutable de `fd-find` s'appelle `fdfind`, pas `fd`.

**2. Régler le nom d'hôte, le fuseau horaire et les locales.**

```bash
sudo hostnamectl set-hostname robot-dev        # corriger aussi la ligne 127.0.1.1 de /etc/hosts
sudo timedatectl set-timezone Europe/Paris
sudo locale-gen en_US.UTF-8 ko_KR.UTF-8
sudo update-locale LANG=en_US.UTF-8
```

Les messages restent en anglais, mais générer aussi `ko_KR.UTF-8` évite les soucis avec les noms de fichiers et
les programmes en coréen.

**3. Faire de vim l'éditeur par défaut.** `visudo`, `crontab -e`, `git commit` et d'autres suivent ce réglage.

```bash
sudo update-alternatives --config editor       # pour tout le système (choisir vim.basic)
```

Par utilisateur, ce sont les variables `EDITOR`/`VISUAL`, que règle `extras.bash` via le script d'installation.

**4. Donner un nom et quelques réglages à git.**

```bash
git config --global user.name "PfClaKr"
git config --global user.email "me@example.com"
git config --global init.defaultBranch main
git config --global push.autoSetupRemote true  # plus besoin de -u au premier push d'une branche (git 2.37+)
```

**5. Créer une clé SSH et l'ajouter à GitHub.**

```bash
ssh-keygen -t ed25519 -C "me@example.com"
cat ~/.ssh/id_ed25519.pub                       # à coller dans GitHub → Settings → SSH keys
ssh -T git@github.com                           # « Hi PfClaKr! ... » : ça marche
```

**6. (Serveur) N'accepter que les connexions SSH par clé.** Plutôt que de modifier `/etc/ssh/sshd_config`, on
ajoute un fichier dans le répertoire d'inclusion :

```bash
sudo tee /etc/ssh/sshd_config.d/10-local.conf <<'EOF'
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
EOF
sudo sshd -t && sudo systemctl restart ssh
```

Quand une option apparaît plusieurs fois, `sshd_config` retient la **première** valeur. Les images cloud d'Ubuntu
contiennent parfois un `50-cloud-init.conf` avec `PasswordAuthentication yes` : le nom du fichier doit donc être
trié avant (`10-`). Avant de redémarrer, vérifiez depuis un autre terminal que la connexion par clé fonctionne.
Les sessions déjà ouvertes ne sont pas coupées.

**7. (Serveur) Activer le pare-feu et les mises à jour de sécurité automatiques.**

```bash
sudo ufw allow OpenSSH && sudo ufw enable
sudo apt install -y unattended-upgrades && sudo dpkg-reconfigure -plow unattended-upgrades
```

Lancer `ufw enable` avant d'autoriser SSH coupe immédiatement une session distante.

**8. Appliquer setting_linux.**

```bash
curl -fsSL https://raw.githubusercontent.com/PfClaKr/setting_linux/main/install.sh | bash
```

Pour garder une copie du dépôt et l'adapter : `git clone https://github.com/PfClaKr/setting_linux.git`, puis `./install.sh`.

## install.sh

Le script d'installation copie les fichiers à leur place et ajoute un bloc à `~/.bashrc`.

| Fichier | Installé dans |
|---|---|
| `vimrc` | `~/.vimrc` |
| `ychun.bash-theme`, `extras.bash` | `~/.config/setting_linux/`, chargés par un bloc à la fin de `~/.bashrc` |
| `ychun.zsh-theme` (avec `--zsh`) | `$ZSH_CUSTOM/themes/`, plus `ZSH_THEME="ychun"` dans `~/.zshrc` |

```bash
./install.sh             # vimrc + prompt bash + extras
./install.sh --zsh       # installe aussi le thème oh-my-zsh
./install.sh --no-vim    # ne touche pas au vimrc
curl -fsSL https://raw.githubusercontent.com/PfClaKr/setting_linux/main/install.sh | bash
```

Le bloc ajouté à `~/.bashrc` :

```bash
# >>> setting_linux >>>
for f in "${XDG_CONFIG_HOME:-$HOME/.config}"/setting_linux/{extras.bash,ychun.bash-theme}; do
  [ -f "$f" ] && . "$f"
done
unset f
# <<< setting_linux <<<
```

- **Sauvegardes :** un fichier existant au contenu différent est conservé avec un horodatage, sous la forme
  `fichier.bak.20261105-141235`.
- **Relançable sans risque :** les fichiers sont simplement recopiés, et les lignes de marquage limitent
  `~/.bashrc` à un seul bloc.
- **Pas besoin de cloner :** lancé par `curl … | bash`, le script n'a aucun fichier à côté de lui ; il les
  télécharge depuis GitHub dans un répertoire temporaire, les installe puis supprime ce répertoire.
  `SETTING_LINUX_RAW` permet de pointer vers un autre emplacement.
- **Les réglages de la distribution restent :** `~/.bashrc` n'est pas remplacé, seulement complété ; seul le
  nécessaire est redéfini par-dessus le `.bashrc` par défaut d'Ubuntu.

Je l'ai vérifié dans des HOME vides ne contenant que le `.bashrc` par défaut d'Ubuntu : une première installation,
une deuxième exécution, un `~/.vimrc` déjà présent, et `--zsh` avec et sans oh-my-zsh. Le `curl … | bash` directement
depuis GitHub, et le chemin wget utilisé sur les machines sans curl, ont donné le même résultat.

## vimrc

Le fichier que `install.sh` copie dans `~/.vimrc` :

```vim
" ~/.vimrc -- installed by setting_linux/install.sh

" A ~/.vimrc stops Vim from loading defaults.vim on its own, so load it here:
" syntax highlighting, filetype indent, incsearch, scrolloff, showcmd, ...
if filereadable(expand('$VIMRUNTIME/defaults.vim'))
	source $VIMRUNTIME/defaults.vim
endif

set mouse=a
set tabstop=4
set shiftwidth=0          " indent by one tabstop (4), not the default 8
set smarttab
set autoindent
set number
set ruler
set laststatus=2
set showmatch
set hlsearch
set ignorecase            " smartcase only works together with ignorecase
set smartcase
set nobackup
set fileencodings=utf-8,cp949   " cp949 also covers EUC-KR files
```

| Réglage | Effet |
|---|---|
| `source $VIMRUNTIME/defaults.vim` | Charge les réglages par défaut de Vim ; expliqué plus bas. |
| `mouse=a` | La souris dans tous les modes : clic pour déplacer le curseur, glisser pour une sélection visuelle, molette pour défiler, glisser les bordures de fenêtres. |
| `tabstop=4` | Un caractère de tabulation du fichier est affiché sur 4 colonnes. |
| `shiftwidth=0` | La largeur d'un niveau d'indentation. `0` signifie « suivre `tabstop` » : un niveau vaut une tabulation (4 colonnes). |
| `smarttab` | <kbd>Tab</kbd> en début de ligne indente de `shiftwidth`. |
| `autoindent` | Une nouvelle ligne reprend l'indentation de la précédente. |
| `number` | Numéros de ligne. |
| `ruler` | La ligne et la colonne du curseur ; avec `laststatus=2`, à droite de la ligne d'état. |
| `laststatus=2` | Toujours afficher la ligne d'état, même avec une seule fenêtre. |
| `showmatch` | Taper une parenthèse fermante fait brièvement sauter le curseur sur l'ouvrante. |
| `hlsearch` | Surligne toutes les correspondances de la dernière recherche. `:noh` efface le surlignage. |
| `ignorecase`, `smartcase` | La recherche ignore la casse, sauf si le motif contient une majuscule. |
| `nobackup` | Pas de sauvegarde `fichier~` à l'écriture. Une option différente du fichier d'échange `.swp`. |
| `fileencodings=utf-8,cp949` | Les encodages essayés à l'ouverture d'un fichier ; expliqué plus bas. |

### Pourquoi charger defaults.vim explicitement

Quand il n'y a **aucun** vimrc utilisateur, Vim charge `$VIMRUNTIME/defaults.vim` : coloration syntaxique
(`syntax on`), plugins et indentation par type de fichier (`filetype plugin indent on`), recherche pendant la frappe
(`incsearch`), cinq lignes de contexte autour du curseur (`scrolloff=5`), `showcmd`, `wildmenu`, retour à la dernière
position à la réouverture d'un fichier, etc. Mais dès que `~/.vimrc` existe, ce fichier n'est plus chargé
automatiquement : les premières lignes le chargent donc à la main. Le `filereadable` évite une erreur sur les
installations minimales sans `defaults.vim` (`vim-tiny`).

C'est aussi ce qui permet à l'indentation de suivre les règles de chaque type de fichier : dans les fichiers C, le
plugin d'indentation C de Vim active `cindent`, tandis que Python et les scripts shell suivent leurs propres règles.
Activer `cindent` globalement dans un vimrc appliquerait les règles du C à tous les fichiers, et comme `#` en C est
une directive du préprocesseur comme `#include` qui va en colonne 0, les commentaires `#` de Python ou du shell se
colleraient à la marge.

### Largeur des tabulations

`shiftwidth` vaut 8 par défaut. Avec seulement `tabstop` à 4, `>>` et l'indentation automatique ajouteraient
8 colonnes, soit deux tabulations de 4, par niveau. Lier les deux avec `shiftwidth=0` fait d'un niveau
d'indentation une tabulation. Du C tapé avec ce vimrc est enregistré ainsi (`^I` est une tabulation) :

```text
int f(void) {
^Iif (x) {
^I^Iy();
^I}
}
```

Dans les fichiers Python, le plugin Python de Vim passe à quatre espaces (`expandtab`, `shiftwidth=4`), comme le
recommande la PEP 8.

### `fileencodings` et les fichiers coréens

Avec la valeur par défaut de Vim (`ucs-bom,utf-8,default,latin1`), un fichier qui n'est pas en UTF-8 s'ouvre en
`latin1` : un fichier EUC-KR contenant « 안녕하세요 » s'affiche `¾È³çÇÏ¼¼¿ä`. `utf-8,cp949` essaie d'abord l'UTF-8
et, si les octets ne sont pas de l'UTF-8 valide, relit le fichier en CP949. L'enregistrement le réécrit dans son
encodage d'origine.

- **L'ordre :** les règles d'octets de l'UTF-8 sont assez strictes pour qu'un fichier dans un autre encodage ne soit
  presque jamais pris pour de l'UTF-8 : l'encodage strict passe en premier. Un fichier purement ASCII est détecté
  comme UTF-8.
- **CP949 plutôt qu'EUC-KR :** EUC-KR ne couvre que 2 350 syllabes hangûl précomposées. Les fichiers coréens créés
  sous Windows sont généralement en CP949, son sur-ensemble, et lus en EUC-KR, une syllabe comme « 똠 » devient
  `<8c>c`. CP949 étant un sur-ensemble d'EUC-KR, les fichiers EUC-KR se lisent aussi.

Ouverture d'un fichier EUC-KR et d'un fichier CP949 contenant « 똠방각하 » :

```text
fileencoding=cp949   안녕하세요
fileencoding=cp949   똠방각하
```

### `mouse=a` et la copie depuis le terminal

Comme Vim prend la souris, la sélection par glisser du terminal ne copie pas dans Vim. Dans la plupart des
terminaux, maintenir <kbd>Maj</kbd> pendant le glisser contourne Vim et sélectionne dans le terminal. Si la copie ne
marche pas sur un serveur en SSH, c'est généralement pour cela.

## Le prompt bash : ychun.bash-theme

Un prompt sur deux lignes qui n'a besoin ni d'oh-my-zsh ni de zsh, seulement de bash. Voici l'écran d'un vrai
terminal (un pty) après quelques commandes :

```text
╭─  root@vm ~                                                 [14:13:09]
╰─➤ cd ~/.config/setting_linux
╭─  root@vm ~/.config/setting_linux                           [14:13:10]
╰─➤ false
╭─  root@vm ~/.config/setting_linux                      1 ↵  [14:13:11]
╰─➤ cd /tmp
╭─  root@vm /tmp                                              [14:13:12]
╰─➤ echo hi; (exit 127)
hi
╭─  root@vm /tmp                                       127 ↵  [14:13:12]
╰─➤ git init -q -b main demo && cd demo && touch f
╭─  root@vm /tmp/demo ‹main*›                                 [14:13:13]
╰─➤ ls  # typing here
```

| Où | Quoi |
|---|---|
| première ligne, à gauche | utilisateur@hôte, répertoire courant (le répertoire personnel en `~`), branche git |
| première ligne, à droite | le code de sortie et `↵` si la commande précédente a échoué, et l'heure d'affichage du prompt |
| seconde ligne | les commandes se tapent après `➤` |

- **En root**, le nom d'utilisateur et `➤` sont en rouge : difficile d'oublier qu'on est dans un shell root ouvert
  avec `sudo -i`.
- **Dans un shell atteint par SSH** (`SSH_CONNECTION` ou `SSH_CLIENT` définis), le nom d'hôte est en rouge : on
  distingue facilement un terminal local d'un terminal connecté à un serveur.
- **La branche git** apparaît en jaune, `‹main›`, ou `‹main*›` avec des modifications non commitées. Sans branche
  (HEAD détachée), elle affiche une étiquette ou un hachage de commit court.
- L'heure est celle de l'affichage du prompt, c'est-à-dire la fin de la commande précédente : pratique pour savoir
  quand une longue commande s'est terminée.
- L'utilisateur et l'hôte utilisent la couleur par défaut du terminal et restent lisibles sur fond clair.

### Comment ça marche

**Les valeurs sont calculées dans `PROMPT_COMMAND` ; `PS1` ne fait que se référer à des variables.** bash exécute
le contenu de `PROMPT_COMMAND` juste avant d'afficher le prompt. Le thème y place `__ychun_prompt` en tête :

```bash
__ychun_prompt() {
  local ec=$?                                  # must stay the first command
  __ychun_git=$(__ychun_git_info)
  ...
  return "$ec"
}
PROMPT_COMMAND="__ychun_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
```

Le code de sortie de la commande précédente, `$?`, est écrasé par n'importe quelle commande : il est récupéré à la
**première ligne** de la fonction. Renvoyer la même valeur à la fin permet aux autres `PROMPT_COMMAND` qui suivent
(celui ajouté par le terminal, par exemple) de voir eux aussi le `$?` d'origine.

**Le nom de branche n'entre jamais dans `PS1` comme du texte.** Si `PS1` était reconstruit à chaque fois avec le nom
de branche collé dedans, bash exécuterait aussi les `$(…)` qu'il contient en développant `PS1` (`promptvars`). Et
git accepte une branche nommée `$(id)` :

```text
$ git check-ref-format --branch '$(id)' && git checkout -q -b '$(id)'
--- nom de branche collé dans PS1
[uid=0(root) gid=0(root) groups=0(root)] $
--- ychun.bash-theme : PS1 ne contient qu'un nom de variable
╭─  root@vm …/inject ‹$(id)›
```

Il suffirait de cloner le dépôt de quelqu'un d'autre et d'y entrer pour exécuter une commande. Avec un nom de
variable comme `${__ychun_git}` dans `PS1`, bash développe la valeur une fois et n'interprète pas son contenu.

**Les codes de couleur sont entourés de `\[ \]`.** readline compte les caractères du prompt pour placer le curseur
et couper les lignes. Des codes de couleur invisibles non entourés de `\[` et `\]` sont comptés aussi, et les longues
commandes ou la navigation dans l'historique coupent alors au mauvais endroit et écrasent du texte.

**La partie droite est dessinée en déplaçant le curseur.** bash n'a pas de prompt à droite comme le `RPROMPT` de
zsh. Avant d'afficher la première ligne, le prompt sauvegarde donc le curseur (`\e7`), se place dans la colonne de
droite (`\e[colG`), affiche le code de sortie et l'heure, puis restaure le curseur (`\e8`). Le tout est entre
`\[ \]`, donc readline lui attribue une largeur nulle. Quand la partie gauche est assez longue pour chevaucher, la
partie droite n'est pas affichée du tout. Dans un terminal de 44 colonnes, l'heure disparaît sur les chemins longs,
et une saisie qui dépasse le bord reste intacte :

```text
╭─  root@vm ~                     [14:13:24]
╰─➤ cd ~/.config/setting_linux
╭─  root@vm ~/.config/setting_linux
╰─➤ false
╭─  root@vm ~/.config/setting_linux
╰─➤ cd /tmp
╭─  root@vm /tmp                  [14:13:27]
╰─➤ echo hi; (exit 127)
hi
╭─  root@vm /tmp           127 ↵  [14:13:27]
╰─➤ git init -q -b main demo && cd demo && t
ouch f
╭─  root@vm /tmp/demo ‹main*›
╰─➤ ls  # typing here
```

**Les informations git viennent de trois commandes git.**

```bash
__ychun_git_info() {
  local ref
  ref=$(GIT_OPTIONAL_LOCKS=0 git symbolic-ref --short -q HEAD 2>/dev/null) ||
    ref=$(GIT_OPTIONAL_LOCKS=0 git describe --tags --exact-match HEAD 2>/dev/null) ||
    ref=$(GIT_OPTIONAL_LOCKS=0 git rev-parse --short HEAD 2>/dev/null) || return 0
  local dirty=
  if [ -n "$(GIT_OPTIONAL_LOCKS=0 git status --porcelain --ignore-submodules=dirty 2>/dev/null | head -n 1)" ]; then
    dirty='*'
  fi
  printf '‹%s%s› ' "$ref" "$dirty"
}
```

Le nom de branche ; à défaut, une étiquette ; à défaut, un hachage de commit court. `GIT_OPTIONAL_LOCKS=0` empêche
`git status` de prendre un verrou pour rafraîchir l'index, afin de ne pas entrer en conflit avec une commande git
lancée dans un autre terminal. Hors d'un dépôt, rien n'est affiché.

## extras.bash

Les réglages bash autres que le prompt :

```bash
HISTSIZE=50000
HISTFILESIZE=100000
HISTCONTROL=ignoreboth:erasedups
HISTTIMEFORMAT='%F %T  '
shopt -s histappend cmdhist

export EDITOR=vim VISUAL=vim

alias c='clear'
alias sysupdate='sudo apt-get update && sudo apt-get upgrade -y && sudo apt-get autoclean && sudo apt-get autoremove -y'
alias ll='ls -alF'
alias grep='grep --color=auto'
```

| Réglage | Effet |
|---|---|
| `HISTSIZE`, `HISTFILESIZE` | Le nombre de commandes gardées en mémoire et dans `~/.bash_history`. Les valeurs par défaut d'Ubuntu sont 1000 et 2000. |
| `HISTCONTROL=ignoreboth:erasedups` | Les commandes commençant par une espace et les répétitions de la précédente ne sont pas gardées, et les anciennes copies d'une commande sont supprimées. Une commande contenant un mot de passe n'est pas enregistrée si on la précède d'une espace. |
| `HISTTIMEFORMAT` | Ajoute l'heure d'exécution de chaque commande à la sortie de `history`. |
| `histappend` | Ajoute au fichier d'historique en quittant au lieu de l'écraser : plusieurs terminaux n'effacent pas l'historique les uns des autres. |
| `cmdhist` | Garde une commande sur plusieurs lignes comme une seule entrée. |

Des fonctions comme `zsh-autosuggestions` et `zsh-syntax-highlighting` de zsh n'existent pas dans bash lui-même. Si
on y tient, l'éditeur de ligne ble.sh pour bash fournit les deux.

## Si vous utilisez zsh : ychun.zsh-theme

Sur les machines avec oh-my-zsh, `./install.sh --zsh` installe un thème zsh au même aspect. Il est basé sur le thème
`bira` d'oh-my-zsh, va dans `$ZSH_CUSTOM/themes/`, et `ZSH_THEME` dans `~/.zshrc` devient `"ychun"`.

```text
╭─  ychun@laptop ~/setting_linux  ‹main*›
╰─➤                                              127 ↵  [21:04:13]
```

La différence avec la version bash est la partie droite : zsh a `RPROMPT`, donc le code de sortie et l'heure sont à
droite de la ligne de saisie.

```zsh
# Based on bira theme

setopt prompt_subst

() {

local PR_USER PR_USER_OP PR_PROMPT PR_HOST

# Check the UID
if [[ $UID -ne 0 ]]; then # normal user
  PR_USER='%F{white}%n%f'
  PR_USER_OP='%F{white}%#%f'
  PR_PROMPT='%f➤ %f'
else # root
  PR_USER='%F{red}%n%f'
  PR_USER_OP='%F{red}%#%f'
  PR_PROMPT='%F{red}➤ %f'
fi

# Check if we are on SSH or not
if [[ -n "$SSH_CLIENT"  ||  -n "$SSH2_CLIENT" ]]; then
  PR_HOST='%F{red}%M%f' # SSH
else
  PR_HOST='%F{white}%M%f' # no SSH
fi


local return_code="%(?..%F{red}%? ↵%f)"

local user_host="  ${PR_USER}%F{cyan}@${PR_HOST}"
local current_dir="%B%F{blue}%~%f%b"
local git_branch='$(git_prompt_info)'

PROMPT="╭─${user_host} ${current_dir} \$(ruby_prompt_info) ${git_branch}
╰─$PR_PROMPT "
RPROMPT="${return_code}  [%*]"

ZSH_THEME_GIT_PROMPT_PREFIX="%F{yellow}‹"
ZSH_THEME_GIT_PROMPT_SUFFIX="› %f"
ZSH_THEME_RUBY_PROMPT_PREFIX="%F{red}‹"
ZSH_THEME_RUBY_PROMPT_SUFFIX="›%f"

}
```

À chaque affichage de `PROMPT`, zsh remplace les séquences qui commencent par `%` :

| Séquence | Signification |
|---|---|
| `%n` | nom d'utilisateur |
| `%M` | nom d'hôte complet (`%m` s'arrête au premier `.`) |
| `%~` | répertoire courant, le répertoire personnel abrégé en `~` |
| `%B` … `%b` | début … fin du gras |
| `%F{couleur}` … `%f` | début d'une couleur de texte … retour à la couleur par défaut |
| `%?` | code de sortie de la commande précédente |
| `%(?.vrai.faux)` | la première partie si ce code vaut 0, la seconde sinon |
| `%*` | heure sur 24 heures avec les secondes |

Les guillemets comptent. `PROMPT` est défini entre guillemets doubles : `${user_host}`, `${current_dir}` et
`$PR_PROMPT` sont donc remplacés par leur valeur **au moment de la définition**, ce qui explique que les variables
`PR_*` puissent être `local` à la fonction anonyme `() { … }`. `$(git_prompt_info)`, en revanche, est inséré depuis
une chaîne entre apostrophes et `\$(ruby_prompt_info)` est protégé par un antislash : les deux restent tels quels dans
`PROMPT`. Avec `setopt prompt_subst`, zsh exécute ces `$(…)` à chaque affichage du prompt.

`git_prompt_info` et `ruby_prompt_info` sont des fonctions d'oh-my-zsh : ce thème ne fonctionne pas sans oh-my-zsh.
Le `*` des modifications est la valeur par défaut d'oh-my-zsh (`ZSH_THEME_GIT_PROMPT_DIRTY`), et `ruby_prompt_info`
est vide sans rvm, rbenv ni chruby, ce qui laisse deux espaces entre le répertoire et `‹main›`.
