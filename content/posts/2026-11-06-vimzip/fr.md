+++
title = "vimzip : un environnement Vim qu'il suffit de décompresser"
description = "Une seule archive liée statiquement qui transforme n'importe quelle machine Linux x86-64 en mon environnement Vim complet — sans root, sans gestionnaire de paquets, sans surprise de glibc. Comment c'est construit et pourquoi chaque pièce est là."
tags = ["vim", "linux", "tools"]
+++

Je passe beaucoup de temps sur des machines qui ne sont pas les miennes : une VM fraîche, un poste du cluster 42, le serveur d'un client en SSH. À chaque fois, la même friction — le Vim système est vieux, il n'y a pas de `clangd`, je ne peux rien `apt install`, et `pip`/`npm` sont un champ de mines derrière un proxy. **vimzip** est ma réponse : une archive qu'on décompresse dans son dossier personnel, et on a tout mon éditeur.

```sh
curl -fsSL https://github.com/PfClaKr/vimzip/releases/latest/download/vimzip-linux-x86_64.tar.gz | tar -xz -C ~
~/vimzip/install.sh
vim .
```

Pas de root, aucun paquet système touché. Ça crée un lien `~/.local/bin/vim`, définit `EDITOR=vim`, et c'est tout. Pour désinstaller, on supprime le dossier.

![vim . ouvre l'arbre, le dernier fichier et l'éditeur](layout.png)

([source](https://github.com/PfClaKr/vimzip))

## Pourquoi le lien statique est toute l'astuce

Si « je recopie juste mon Vim » échoue d'habitude, c'est à cause des bibliothèques partagées. Un Vim compilé sur Ubuntu 24.04 est lié à la glibc et à la ncurses de cette machine ; posez-le sur CentOS 7 et il meurt avec `GLIBC_2.34 not found`. vimzip compile donc Vim **statiquement avec musl**, pas glibc :

```sh
PKG_CONFIG=false ./configure CC="$CC" \
  CPPFLAGS="-I$W/nc/include -I$W/nc/include/ncursesw" \
  LDFLAGS="-static -L$W/nc/lib" \
  --with-features=huge --with-tlib=ncursesw --enable-multibyte \
  --disable-gui --without-x --disable-gpm ...
```

ncurses est compilée d'abord, statique elle aussi (`--without-shared`), avec une liste `--with-fallbacks` généreuse (`xterm-256color`, `screen`, `tmux-256color`…) embarquée, pour que le terminal fonctionne même là où `/usr/share/terminfo` manque. Le résultat : un binaire `vim` sans aucune dépendance de bibliothèque partagée — il tourne pareil sur Alpine, une vieille Debian ou un conteneur scratch. Même approche pour les outils compagnons : `ctags`, `fzf`, `ripgrep`, `shellcheck` sont récupérés en binaires de release statiques, à des versions épinglées, jamais depuis l'hôte.

## Pas de gestionnaire de plugins, juste `pack/`

Vim 8+ a une disposition de paquets native, donc vimzip n'a besoin ni de vim-plug ni de Lazy. Les plugins vivent sous `pack/vimzip/start/` et se chargent tout seuls ; dans le dépôt ce sont des sous-modules git, et la release livre simplement les copies extraites, `.git`/tests retirés. L'ensemble est volontairement petit et 100 % Vimscript natif (pas de Node, pas d'hôte Python) :

- **ALE** — linting et client LSP (utilise `clangd` quand l'hôte l'a, sinon retombe sur le compilateur système)
- **gutentags** + **tagbar** — ctags automatiques et plan des symboles
- **fzf.vim**, **nerdtree** (+git), **gitgutter**, **fugitive**, **lightline** (+bufferline)

plus les intégrés de Vim qu'on oublie : `comment`, `editorconfig`, `matchit`, `termdebug` et `osc52`.

## Les choix de config qui comptent vraiment

Un binaire portable, c'est la moitié ; l'autre moitié, c'est qu'il se comporte comme un éditeur déjà réglé.

**`vim .` est un projet, pas un fichier.** Démarrer dans un dossier restaure une session par dossier (splits ouverts, dernier fichier touché) et ouvre l'arbre. La session et le reste — historique d'annulation, swap, tags — vivent dans `~/.cache/vimzip`, jamais éparpillés dans le projet.

**Sauvegarde auto, parce que j'oublie.** Quitter le mode insertion, toute modification en mode normal, perdre le focus ou changer de buffer déclenchent un `update` silencieux. Avec l'annulation persistante (`undofile`), ça ressemble plus à un éditeur moderne qu'au Vim classique.

**Presse-papiers par-dessus SSH.** Les copies (`y`) sont renvoyées au presse-papiers système via les séquences OSC 52, donc un `y` sur une machine distante atterrit dans votre presse-papiers *local* si le terminal le gère — sans forwarding X, sans `xclip`.

**Aller à la définition, en douceur.** Avec `clangd`, c'est un vrai saut LSP ; sans lui, vimzip retombe sur les tags du projet, et si même ceux-là manquent, il génère à la volée les tags des *en-têtes système inclus par le fichier courant* (en lisant `cc -M`), pour que `gd` sur `malloc` atterrisse quand même dans l'en-tête de la libc.

**Complétion depuis tout le projet.** `Ctrl+n` propose des mots collectés par ripgrep sur tout le dépôt git dans une tâche de fond — pas cher, sans indexeur, et ça couvre les fichiers pas encore ouverts.

Il y a même un panneau d'agent IA (`Space a`) qui lance le premier trouvé parmi `claude`/`codex`/`gemini`/`aider`/`opencode`. [Les raccourcis complets, avec captures d'écran, font l'objet d'un autre article](/fr/posts/vimzip-keymap/).

## Le garder à jour sans y toucher

Les versions des outils sont épinglées dans un seul `versions.env`. Une GitHub Action hebdomadaire résout les derniers tags amont, et si quelque chose a changé elle reconstruit toute l'archive, lance un test de fumée (démarrer Vim sans interface, vérifier que les plugins sont chargés, générer les tags d'aide) et ouvre une PR. Fusionner la PR publie une nouvelle release. Le truc reste donc à jour tout seul, et chaque release en est une qui a réellement démarré.

## Pourquoi j'aime ça

La philosophie est ennuyeuse exprès : **un fichier, pas d'étape d'installation au-delà du décompressage, pas d'état hors de deux dossiers, et le supprimer ne laisse aucune trace.** Ça ne cherche pas à être un Neovim-avec-tout. C'est la plus petite chose qui fait qu'une machine Linux inconnue ressemble à la mienne, et elle marche pareil que la machine date de 2014 ou d'hier.
