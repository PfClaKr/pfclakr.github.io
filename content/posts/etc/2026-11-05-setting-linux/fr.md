+++
title = "setting_linux : configurer une nouvelle machine Linux, un vimrc, et passer de zsh à bash"
description = "La liste de ce que je configure sur une machine Linux neuve, et le dépôt setting_linux que j'emporte : un vimrc, un prompt et un script d'installation. Comment les pièges du vimrc (largeur des tabulations, smartcase, commentaires #, EUC-KR) ont été corrigés, et ce qui a changé en portant le thème oh-my-zsh vers bash pur, le tout vérifié en l'exécutant."
tags = ["linux", "vim", "bash", "zsh", "shell", "dotfiles"]
+++

Les fichiers que je copie en premier sur chaque nouvelle machine Linux sont rassemblés dans le dépôt
[setting_linux](https://github.com/PfClaKr/setting_linux). Au départ, il n'y avait qu'un `vimrc`, un thème oh-my-zsh
et un `.zshrc` ; pour retrouver le même environnement sur des serveurs et des conteneurs sans zsh, j'ai porté le
prompt vers bash pur et ajouté un script qui applique le tout d'un coup.

```text
setting_linux/
├── install.sh          applique vimrc + prompt d'un coup
├── vimrc               → ~/.vimrc
├── ychun.bash-theme    le prompt bash
├── extras.bash         réglages de l'historique, alias
├── ychun.zsh-theme     le thème oh-my-zsh d'origine (installé avec --zsh)
└── .zshrc
```

Cet article parcourt la liste pour une machine neuve, le script d'installation, le vimrc, puis le thème zsh et son
portage vers bash. Tout a été vérifié sous Ubuntu 24.04 avec Vim 9.1, bash 5.2 et zsh 5.9.

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
git clone git@github.com:PfClaKr/setting_linux.git && ./setting_linux/install.sh
```

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
une deuxième exécution, un `~/.vimrc` déjà présent, et `--zsh` avec et sans oh-my-zsh.

## vimrc

### La première version

Le vimrc d'origine du dépôt faisait 18 lignes :

```vim
if has("syntax")
	syntax on
endif

set mouse=a
set ts=4
set hlsearch
set nobackup
set nu
set autoindent
set cindent
set laststatus=2
set showmatch
set smartcase
set smarttab
set smartindent
set ruler
set fileencodings=utf8,euc-kr
```

| Option | Effet |
|---|---|
| `syntax on` | Active la coloration syntaxique. `has("syntax")` vérifie que Vim a été compilé avec cette fonctionnalité, pour éviter une erreur sur les versions minimales (`vim-tiny`). |
| `mouse=a` | La souris dans tous les modes : clic pour déplacer le curseur, glisser pour une sélection visuelle, molette pour défiler, glisser les bordures de fenêtres. |
| `ts=4` | `tabstop` : un caractère de tabulation du fichier est affiché sur 4 colonnes. |
| `hlsearch` | Surligne toutes les correspondances de la dernière recherche. `:noh` efface le surlignage. |
| `nobackup` | Pas de sauvegarde `fichier~` à l'écriture. C'est déjà la valeur par défaut, et c'est une option différente du fichier d'échange `.swp`. |
| `nu` | `number` : numéros de ligne. |
| `autoindent` | Une nouvelle ligne reprend l'indentation de la précédente. |
| `cindent` | Indente selon les règles du C : un niveau après `{`, retour à la bonne colonne sur `}`. |
| `smartindent` | Une règle plus simple qui indente après `{` ou des mots-clés comme `if` et `for`. Sans effet tant que `cindent` est actif. |
| `smarttab` | <kbd>Tab</kbd> en début de ligne indente de `shiftwidth` au lieu de `tabstop`. |
| `laststatus=2` | Toujours afficher la ligne d'état, même avec une seule fenêtre. |
| `ruler` | Affiche la ligne et la colonne du curseur ; avec `laststatus=2`, à droite de la ligne d'état. |
| `showmatch` | Taper une parenthèse fermante fait brièvement sauter le curseur sur l'ouvrante. |
| `smartcase` | Recherche sensible à la casse quand le motif contient une majuscule. Comme on le verra, elle ne fait rien seule. |
| `fileencodings=utf8,euc-kr` | Les encodages essayés à l'ouverture d'un fichier : UTF-8 d'abord, EUC-KR si les octets ne sont pas de l'UTF-8 valide. |

Le bloc `if has("syntax")`, `smartcase` et `mouse=a` ont la même forme que les exemples commentés du
`/etc/vim/vimrc` de Debian/Ubuntu : le fichier est essentiellement une sélection de lignes de ce modèle. En
l'exécutant vraiment, cinq endroits ne faisaient pas ce qui était prévu.

### Un `~/.vimrc` désactive defaults.vim

Quand il n'y a **aucun** vimrc utilisateur, Vim charge `$VIMRUNTIME/defaults.vim`, qui active `incsearch`
(recherche pendant la frappe), `scrolloff=5` (cinq lignes de contexte autour du curseur), `showcmd`, `wildmenu`,
`filetype plugin indent on`, etc. Dès que `~/.vimrc` existe, ce fichier n'est plus chargé. Si la recherche a cessé de
suivre la frappe après la création de votre premier vimrc, c'est pour cette raison. Sur les systèmes Debian, le
vimrc système charge toujours `debian.vim`, qui conserve l'essentiel comme `nocompatible` et
`backspace=indent,eol,start`.

### Largeur des tabulations : `ts=4`, mais des indentations de 8 colonnes

Seul `tabstop` passait à 4 ; `shiftwidth` restait à sa valeur par défaut, 8. Or `cindent`, `>>` et la touche
<kbd>Tab</kbd> avec `smarttab` indentent tous de `shiftwidth` : un niveau d'indentation devenait deux tabulations de
4 colonnes. Du C tapé avec ce vimrc était enregistré ainsi (`^I` est une tabulation) :

```text
int f(void) {
^I^Iif (x) {
^I^I^I^Iy();
^I^I}
^I^Ireturn 0;
}
```

À l'écran chaque niveau fait 8 colonnes ; dans un autre éditeur ou avec `cat` et des tabulations de 8, il en fait 16.

### `smartcase` a besoin de `ignorecase`

`smartcase` est une exception à `ignorecase` : « ignorer la casse, sauf si le motif contient une majuscule ». Sans
`ignorecase`, elle ne fait rien. Les deux lignes se suivent dans le modèle Debian, et seule `smartcase` avait été reprise.

### Un `cindent` global et les commentaires `#`

`cindent` est une option locale au tampon, mais `set` dans le vimrc fixe la valeur dont hérite chaque nouveau tampon,
et l'indentation par type de fichier n'était pas activée. Python, les scripts shell et le YAML étaient donc indentés
selon les règles du C. Le symptôme le plus visible concerne les lignes qui commencent par `#` : en C, c'est une
directive du préprocesseur comme `#include`, qui va en colonne 0. Taper un commentaire dans un bloc Python donne :

```text
for i in x:
^I^Ia = 1
# note
^I^Ib = 2
```

La ligne se colle à la marge dès que `#` est tapé. `smartindent` a la même règle, donc désactiver `cindent` seul ne
suffit pas.

### `fileencodings` et les fichiers coréens

Avec la valeur par défaut (`ucs-bom,utf-8,default,latin1`), un fichier qui n'est pas en UTF-8 s'ouvre en `latin1` :
un fichier EUC-KR contenant « 안녕하세요 » s'affiche `¾È³çÇÏ¼¼¿ä`. Le premier vimrc évitait cela en retentant en
EUC-KR une lecture UTF-8 échouée, et l'enregistrement réécrit le fichier dans son encodage d'origine. Les règles
d'octets de l'UTF-8 sont assez strictes pour qu'un fichier dans un autre encodage ne soit presque jamais pris pour
de l'UTF-8 : mettre l'encodage strict en premier est le bon ordre.

La faille : EUC-KR ne couvre que 2 350 syllabes hangûl précomposées. Les fichiers coréens créés sous Windows sont
généralement en CP949, son sur-ensemble, et se cassent dès qu'ils contiennent une syllabe comme « 똠 » :

```text
fileencoding=euc-kr   <8c>c방각하
```

### `mouse=a` et la copie depuis le terminal

Comme Vim prend la souris, la sélection par glisser du terminal ne fonctionne plus dans Vim. Dans la plupart des
terminaux, maintenir <kbd>Maj</kbd> pendant le glisser contourne Vim et sélectionne dans le terminal. Si la copie ne
marche pas sur un serveur en SSH, c'est généralement à cause de cette option. C'est moins un piège qu'un
comportement à connaître : l'option reste dans la version actuelle.

### La version actuelle

Une fois tout cela appliqué, le vimrc du dépôt est devenu :

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

- Charger `defaults.vim` explicitement ramène `syntax on`, `incsearch`, `scrolloff` et l'indentation par type de
  fichier. Le `filereadable` évite une erreur sur les installations minimales sans `defaults.vim` (`vim-tiny`).
- `shiftwidth=0` signifie « suivre `tabstop` » : un niveau d'indentation vaut une tabulation.
- `cindent` et `smartindent` disparaissent au profit de l'indentation par type de fichier : dans les fichiers C, le
  plugin d'indentation C de Vim active `cindent`, tandis que Python et le shell suivent leurs propres règles.
- `cp949` est un sur-ensemble d'EUC-KR : les fichiers EUC-KR se lisent toujours.

Les mêmes tests, à nouveau : le C s'indente d'une tabulation par niveau, le `# note` d'un script shell reste où il a
été tapé, et les deux fichiers coréens s'ouvrent correctement.

```text
int f(void) {
^Iif (x) {
^I^Iy();
^I}
}

fileencoding=cp949   안녕하세요
fileencoding=cp949   똠방각하
```

## Le thème zsh : ychun.zsh-theme

Le thème est basé sur `bira`, fourni par oh-my-zsh. `./install.sh --zsh` le place dans `$ZSH_CUSTOM/themes/`
(en général `~/.oh-my-zsh/custom/themes/`) et le sélectionne avec `ZSH_THEME="ychun"` dans `.zshrc`. À l'écran :

```text
╭─  ychun@laptop ~/setting_linux  ‹main*›
╰─➤                                              127 ↵  [21:04:13]
```

C'est un prompt sur deux lignes. La première contient l'utilisateur, l'hôte, le répertoire courant et la branche
git ; les commandes se tapent après le `➤` de la seconde. À droite : le code de sortie de la commande précédente
(seulement en cas d'échec) et l'heure.

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

### Les séquences du prompt

À chaque affichage de `PROMPT`, zsh remplace les séquences qui commencent par `%` :

| Séquence | Signification |
|---|---|
| `%n` | nom d'utilisateur |
| `%M` | nom d'hôte complet (`%m` s'arrête au premier `.`) |
| `%~` | répertoire courant, le répertoire personnel abrégé en `~` |
| `%#` | `%` pour un utilisateur normal, `#` pour root |
| `%B` … `%b` | début … fin du gras |
| `%F{couleur}` … `%f` | début d'une couleur de texte … retour à la couleur par défaut |
| `%?` | code de sortie de la commande précédente |
| `%(?.vrai.faux)` | la première partie si ce code vaut 0, la seconde sinon |
| `%*` | heure sur 24 heures avec les secondes |

Le `%(?..%F{red}%? ↵%f)` de `return_code` se lit « rien en cas de succès, le code et `↵` en rouge en cas
d'échec ». `[%*]` est l'heure d'affichage du prompt, c'est-à-dire la fin de la commande précédente : pratique pour
savoir quand une longue commande s'est terminée.

### Colorer root et les sessions SSH

- **Quand l'UID vaut 0**, le nom d'utilisateur et `➤` passent en rouge : difficile d'oublier qu'on est dans un shell
  root ouvert avec `sudo -i`.
- **Quand `SSH_CLIENT` ou `SSH2_CLIENT` est défini**, le nom d'hôte passe en rouge. OpenSSH définit `SSH_CLIENT`
  dans les shells distants : la couleur du nom d'hôte distingue d'un coup d'œil un terminal local d'un terminal
  connecté à un serveur. `SSH2_CLIENT` est la variable d'une implémentation SSH commerciale.

`PR_USER_OP` (`%#`) est défini de la même façon mais n'est jamais utilisé dans `PROMPT` ; c'est un reste du
remplacement des symboles `$`/`#` de bira par `➤`.

### Guillemets et `prompt_subst`

`PROMPT` mélange des parties dont la valeur est fixée à la définition et des parties recalculées à chaque affichage.

- `${user_host}`, `${current_dir}` et `$PR_PROMPT` sont entre guillemets doubles, donc remplacés par leur valeur
  **au moment de la définition**. C'est pourquoi les variables `PR_*` peuvent être `local` à la fonction anonyme
  `() { … }`.
- `$(git_prompt_info)` est inséré depuis une chaîne entre apostrophes, et `\$(ruby_prompt_info)` est protégé par un
  antislash. Les deux restent dans `PROMPT` sous la forme littérale `$(…)`.
- Avec `setopt prompt_subst`, zsh exécute les `$(…)` de `PROMPT` à chaque affichage.

Afficher la valeur stockée de `PROMPT` le montre (exécuté en root) :

```text
╭─  %F{red}%n%f%F{cyan}@%F{white}%M%f %B%F{blue}%~%f%b $(ruby_prompt_info) $(git_prompt_info)
╰─%F{red}➤ %f 
```

`git_prompt_info` et `ruby_prompt_info` sont fournies par oh-my-zsh. Avec des fichiers modifiés, la branche devient
`‹main*›` ; le thème ne définit pas `ZSH_THEME_GIT_PROMPT_DIRTY`, donc la valeur par défaut d'oh-my-zsh, `*`,
s'applique (bira met un `●` rouge). `ruby_prompt_info` est vide sans rvm, rbenv ni chruby, ce qui laisse deux espaces
entre le répertoire et `‹main›`.

## Le portage vers bash : ychun.bash-theme

Le thème zsh ne fonctionne pas sans oh-my-zsh : `git_prompt_info` est une fonction d'oh-my-zsh, et c'est oh-my-zsh
qui charge les fichiers de thème. Les serveurs, les conteneurs et les cartes embarquées n'ont souvent même pas zsh :
j'ai donc reconstruit le même aspect avec bash seul. Voici l'écran d'un vrai terminal (un pty) après quelques commandes :

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

Le fichier complet est `ychun.bash-theme` dans le dépôt. Cinq choses ont changé avec le portage.

**1. `PROMPT_COMMAND` au lieu de `prompt_subst`.** bash exécute le contenu de `PROMPT_COMMAND` juste avant
d'afficher le prompt. C'est là que les informations git, l'heure et le code de sortie sont calculés dans des
variables ; `PS1` ne fait que s'y référer.

```bash
__ychun_prompt() {
  local ec=$?                                  # must stay the first command
  __ychun_git=$(__ychun_git_info)
  ...
  return "$ec"
}
PROMPT_COMMAND="__ychun_prompt${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
```

Le code de sortie de la commande précédente, `$?`, est écrasé par n'importe quelle commande : il faut le récupérer
à la **première ligne** de la fonction. Renvoyer la même valeur à la fin permet aux autres `PROMPT_COMMAND` qui
suivent (celui ajouté par le terminal, par exemple) de voir eux aussi le `$?` d'origine.

**2. Le nom de branche n'entre jamais directement dans `PS1`.** Si `PS1` est reconstruit à chaque fois avec le nom
de branche collé dedans comme du texte, bash exécute aussi les `$(…)` qu'il contient en développant `PS1`
(`promptvars`). Et git accepte une branche nommée `$(id)` :

```text
$ git check-ref-format --branch '$(id)' && git checkout -q -b '$(id)'
--- nom de branche collé dans PS1
[uid=0(root) gid=0(root) groups=0(root)] $
--- ychun.bash-theme : PS1 ne contient qu'un nom de variable
╭─  root@vm …/inject ‹$(id)›
```

Il suffirait de cloner le dépôt de quelqu'un d'autre et d'y entrer pour exécuter une commande. Avec un nom de
variable comme `${__ychun_git}` dans `PS1`, bash développe la valeur une fois et n'interprète pas son contenu. Le
thème zsh n'avait pas ce problème, car la sortie de `$(git_prompt_info)` n'est pas réinterprétée.

**3. Les codes de couleur sont entourés de `\[ \]`.** readline compte les caractères du prompt pour placer le curseur
et couper les lignes. Des codes de couleur invisibles non entourés de `\[` et `\]` sont comptés aussi, et les longues
commandes ou la navigation dans l'historique coupent alors au mauvais endroit et écrasent du texte. zsh calcule
lui-même la largeur de `%F{…}` : ce n'était pas nécessaire là-bas.

**4. Il n'y a pas de prompt à droite.** bash n'a pas de `RPROMPT` : avant d'afficher la première ligne, le prompt
sauvegarde le curseur (`\e7`), se place dans la colonne de droite (`\e[colG`), affiche le code de sortie et l'heure,
puis restaure le curseur (`\e8`). Le tout est entre `\[ \]`, donc readline lui attribue une largeur nulle. C'est
pourquoi, contrairement à zsh, l'heure est sur la première ligne et non sur la ligne de saisie. Quand la partie
gauche est assez longue pour chevaucher, la partie droite n'est pas affichée du tout. Dans un terminal de 44
colonnes, l'heure disparaît sur les chemins longs, et une saisie qui dépasse le bord reste intacte :

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

**5. Les informations git sont calculées directement.** Trois commandes git remplacent le `git_prompt_info`
d'oh-my-zsh :

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

Le nom de branche ; à défaut (HEAD détachée), une étiquette ; à défaut, un hachage de commit court.
`GIT_OPTIONAL_LOCKS=0` empêche `git status` de prendre un verrou pour rafraîchir l'index, afin de ne pas entrer en
conflit avec une commande git lancée dans un autre terminal ; oh-my-zsh utilise la même variable pour la même raison.

Le reste de ce que fournissait zsh :

- Les alias du `.zshrc` (`c`, `sysupdate`) et les réglages de l'historique sont passés dans `extras.bash` :
  50 000 lignes, doublons supprimés, horodatage (`HISTTIMEFORMAT`) et `histappend` pour que plusieurs terminaux ne
  s'écrasent pas mutuellement.
- Les plugins oh-my-zsh comme `zsh-autosuggestions` et `zsh-syntax-highlighting` n'ont pas d'équivalent dans bash
  lui-même. Si on y tient vraiment, l'éditeur de ligne ble.sh pour bash fournit les deux.
- L'utilisateur et l'hôte utilisent la couleur par défaut du terminal au lieu du blanc du thème zsh (`%F{white}`),
  difficile à lire sur un terminal à fond clair.
