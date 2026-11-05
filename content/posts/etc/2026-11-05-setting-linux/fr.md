+++
title = "setting_linux : un vimrc de 18 lignes et un thème zsh, ligne par ligne"
description = "Le vimrc et le thème oh-my-zsh basé sur bira que j'emporte sur chaque nouvelle machine Linux, expliqués ligne par ligne : ce que fait chaque option, les pièges trouvés en les exécutant vraiment (largeur des tabulations, smartcase, commentaires #, EUC-KR) et la façon dont le prompt est dessiné."
tags = ["linux", "vim", "zsh", "shell", "dotfiles"]
+++

Les fichiers que je copie en premier sur chaque nouvelle machine Linux sont rassemblés dans le dépôt
[setting_linux](https://github.com/PfClaKr/setting_linux) : `vimrc`, `ychun.zsh-theme` et `.zshrc`.
Cet article traite du `vimrc` et du thème. Ce sont des fichiers courts, mais chaque option en fait plus
qu'il n'y paraît, et les exécuter a révélé quelques endroits où ils ne font pas ce qui était prévu.

Tout ce qui suit a été vérifié avec Vim 9.1, zsh 5.9 et un oh-my-zsh récent.

## vimrc

Le fichier complet, copié dans `~/.vimrc` :

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

### Ce que fait chaque option

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
`/etc/vim/vimrc` de Debian/Ubuntu : le fichier est essentiellement une sélection de lignes de ce modèle.

### Un `~/.vimrc` désactive defaults.vim

Quand il n'y a **aucun** vimrc utilisateur, Vim charge `$VIMRUNTIME/defaults.vim`, qui active
`incsearch` (recherche pendant la frappe), `scrolloff=5` (cinq lignes de contexte autour du curseur),
`showcmd`, `wildmenu`, `filetype plugin indent on`, etc. Dès que `~/.vimrc` existe, ce fichier n'est plus
chargé. Si la recherche a cessé de suivre la frappe après la création de votre premier vimrc, c'est pour
cette raison.

Sur les systèmes Debian, le vimrc système charge toujours `debian.vim`, qui conserve l'essentiel comme
`nocompatible` et `backspace=indent,eol,start`. Pour le reste, ajoutez `source $VIMRUNTIME/defaults.vim`
en haut du vimrc.

### Largeur des tabulations : `ts=4`, mais des indentations de 8 colonnes

Le fichier passe `tabstop` à 4 sans toucher à `shiftwidth`, dont la valeur par défaut est 8. Or
`cindent`, `>>` et la touche <kbd>Tab</kbd> avec `smarttab` indentent tous de `shiftwidth` : un niveau
d'indentation devient deux tabulations de 4 colonnes. Du C tapé avec ce vimrc est enregistré ainsi (`^I`
est une tabulation) :

```text
int f(void) {
^I^Iif (x) {
^I^I^I^Iy();
^I^I}
^I^Ireturn 0;
}
```

À l'écran chaque niveau fait 8 colonnes ; dans un autre éditeur ou avec `cat` et des tabulations de 8,
il en fait 16. Ajouter `set shiftwidth=4`, ou `set shiftwidth=0` pour suivre `tabstop`, fait d'un niveau
une seule tabulation.

### `smartcase` a besoin de `ignorecase`

`smartcase` est une exception à `ignorecase` : « ignorer la casse, sauf si le motif contient une
majuscule ». Sans `ignorecase`, elle ne fait rien : avec ce fichier, les recherches sont toujours
sensibles à la casse. Les deux lignes se suivent dans le modèle Debian, et seule `smartcase` a été
reprise. Pour obtenir le comportement voulu, ajoutez `set ignorecase`.

### Un `cindent` global et les commentaires `#`

`cindent` est une option locale au tampon, mais `set` dans le vimrc fixe la valeur dont hérite chaque
nouveau tampon, et l'indentation par type de fichier (`filetype indent on`) n'est pas activée. Python,
les scripts shell et le YAML sont donc indentés selon les règles du C. Le symptôme le plus visible
concerne les lignes qui commencent par `#` : en C, c'est une directive du préprocesseur comme
`#include`, qui va en colonne 0. Taper un commentaire dans un bloc Python donne :

```text
for i in x:
^I^Ia = 1
# note
^I^Ib = 2
```

La ligne se colle à la marge dès que `#` est tapé. `smartindent` a la même règle, donc désactiver
`cindent` seul ne suffit pas. Retirer les deux et utiliser `filetype plugin indent on` applique les
règles du C aux fichiers C et celles de Python aux fichiers Python.

### `fileencodings` et les fichiers coréens

Avec la valeur par défaut (`ucs-bom,utf-8,default,latin1`), un fichier qui n'est pas en UTF-8 s'ouvre en
`latin1`. Un fichier EUC-KR contenant « 안녕하세요 » s'affiche ainsi :

```text
fileencoding=latin1   ¾È³çÇÏ¼¼¿ä
```

Avec ce vimrc, une lecture UTF-8 qui échoue est retentée en EUC-KR et s'affiche correctement, et
l'enregistrement réécrit le fichier dans son encodage d'origine (`fileencoding=euc-kr`). L'ordre compte :
les règles d'octets de l'UTF-8 sont assez strictes pour qu'un fichier dans un autre encodage ne soit
presque jamais pris pour de l'UTF-8, donc l'encodage strict passe en premier. Un fichier purement ASCII
est détecté comme UTF-8.

La faille : EUC-KR ne couvre que 2 350 syllabes hangûl précomposées. Les fichiers coréens créés sous
Windows sont généralement en CP949, son sur-ensemble, et se cassent dès qu'ils contiennent une syllabe
comme « 똠 » :

```text
fileencoding=euc-kr   <8c>c방각하
```

Utiliser `cp949` à la place de `euc-kr` lit correctement les fichiers EUC-KR comme CP949.

### `mouse=a` et la copie depuis le terminal

Comme Vim prend la souris, la sélection par glisser du terminal ne fonctionne plus dans Vim. Dans la
plupart des terminaux, maintenir <kbd>Maj</kbd> pendant le glisser contourne Vim et sélectionne dans le
terminal. Si la copie ne marche pas sur un serveur en SSH, c'est généralement à cause de cette option.

### Avec tout cela appliqué

```vim
source $VIMRUNTIME/defaults.vim   " incsearch, scrolloff, filetype plugin indent on, ...

set mouse=a
set tabstop=4 shiftwidth=0        " un niveau d'indentation = une tabulation
set hlsearch
set number
set autoindent
set laststatus=2
set showmatch
set ignorecase smartcase
set smarttab
set fileencodings=utf-8,cp949
```

`defaults.vim` active `syntax on`, `ruler` et l'indentation par type de fichier : les lignes `cindent`,
`smartindent` et le bloc `syntax on` disparaissent.

## Le thème zsh : ychun.zsh-theme

Le thème est basé sur `bira`, fourni par oh-my-zsh. Il se place dans `$ZSH_CUSTOM/themes/` (en général
`~/.oh-my-zsh/custom/themes/`) et se choisit avec `ZSH_THEME="ychun"` dans `.zshrc`. À l'écran :

```text
╭─  ychun@laptop ~/setting_linux  ‹main*›
╰─➤                                              127 ↵  [21:04:13]
```

C'est un prompt sur deux lignes. La première contient l'utilisateur, l'hôte, le répertoire courant et la
branche git ; les commandes se tapent après le `➤` de la seconde. À droite : le code de sortie de la
commande précédente (seulement en cas d'échec) et l'heure.

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

À chaque affichage de `PROMPT`, zsh remplace les séquences qui commencent par `%`. Celles utilisées ici :

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
d'échec » : le nombre à droite n'apparaît que si quelque chose a échoué. `[%*]` est l'heure d'affichage du
prompt, c'est-à-dire la fin de la commande précédente : pratique pour savoir quand une longue commande
s'est terminée.

### Colorer root et les sessions SSH

Les deux `if` du début sont le cœur du thème.

- **Quand l'UID vaut 0**, le nom d'utilisateur et `➤` passent en rouge : difficile d'oublier qu'on est
  dans un shell root ouvert avec `sudo -i`.
- **Quand `SSH_CLIENT` ou `SSH2_CLIENT` est défini**, le nom d'hôte passe en rouge. OpenSSH définit
  `SSH_CLIENT` dans les shells distants : la couleur du nom d'hôte distingue d'un coup d'œil un terminal
  local d'un terminal connecté à un serveur. `SSH2_CLIENT` est la variable d'une implémentation SSH
  commerciale.

`PR_USER_OP` (`%#`) est défini de la même façon mais n'est jamais utilisé dans `PROMPT` ; c'est un reste
du remplacement des symboles `$`/`#` de bira par `➤`.

### Guillemets et `prompt_subst`

La partie déroutante du thème, ce sont les guillemets. `PROMPT` mélange des parties dont la valeur est
fixée à la définition et des parties recalculées à chaque affichage.

```zsh
local git_branch='$(git_prompt_info)'          # apostrophes : stocké tel quel
PROMPT="╭─${user_host} ${current_dir} \$(ruby_prompt_info) ${git_branch}
╰─$PR_PROMPT "                                 # guillemets doubles : développé maintenant
```

- `${user_host}`, `${current_dir}` et `$PR_PROMPT` sont entre guillemets doubles, donc remplacés par leur
  valeur **au moment de la définition**. C'est pourquoi les variables `PR_*` peuvent être `local` à la
  fonction anonyme `() { … }` : quand la fonction se termine et qu'elles disparaissent, `PROMPT` contient
  déjà leurs valeurs. La fonction anonyme évite aussi de polluer l'espace de noms global du shell.
- `$(git_prompt_info)` est inséré depuis une chaîne entre apostrophes, et `\$(ruby_prompt_info)` est
  protégé par un antislash. Les deux restent dans `PROMPT` sous la forme littérale `$(…)`.
- Avec `setopt prompt_subst`, zsh exécute les `$(…)` de `PROMPT` à chaque affichage : les informations
  git suivent dès qu'on change de répertoire ou de branche.

Afficher la valeur stockée de `PROMPT` le montre (exécuté en root) :

```text
╭─  %F{red}%n%f%F{cyan}@%F{white}%M%f %B%F{blue}%~%f%b $(ruby_prompt_info) $(git_prompt_info)
╰─%F{red}➤ %f 
```

### Informations git et ruby

`git_prompt_info` et `ruby_prompt_info` sont fournies par oh-my-zsh ; le thème ne fixe que les chaînes
qui les entourent, via les variables `ZSH_THEME_*`. Dans un dépôt git, on obtient un `‹main›` jaune, et
rien en dehors.

- **Avec des fichiers modifiés**, cela devient `‹main*›`. Le thème ne définit pas
  `ZSH_THEME_GIT_PROMPT_DIRTY`, donc la valeur par défaut d'oh-my-zsh, `*`, s'applique ; bira y met un
  `●` rouge.
- `ruby_prompt_info` n'affiche une version de Ruby qu'en présence de rvm, rbenv ou chruby. Sur la plupart
  des machines elle est vide, ce qui laisse deux espaces entre le répertoire et `‹main›`, comme dans
  l'exemple plus haut.
- Les versions récentes d'oh-my-zsh calculent `git_prompt_info` de façon asynchrone : dans un gros dépôt
  où `git status` est lent, le prompt s'affiche d'abord et la branche apparaît un instant plus tard.

### Ce qui change par rapport à bira

| | bira | ychun |
|---|---|---|
| utilisateur@hôte | vert gras, rouge pour root | utilisateur blanc, `@` cyan, hôte rouge en SSH |
| symbole de saisie | `$` / `#` | `➤` (rouge pour root) |
| à droite | code de sortie en échec | code de sortie en échec + heure |
| couleurs | `%{$fg[red]%}` … `%{$reset_color%}` | `%F{red}` … `%f` |
| autres infos | conda, virtualenv, hg, kube-ps1 | git et ruby seulement |

La syntaxe des couleurs n'est pas qu'une question de goût. `$fg[red]` insère directement la séquence
d'échappement du terminal : il faut l'entourer de `%{ … %}`, sinon zsh compte mal la largeur du prompt.
`%F{…}` est géré par zsh lui-même, n'a pas besoin d'être entouré et accepte aussi les numéros de la
palette 256 couleurs, comme `%F{208}`.
