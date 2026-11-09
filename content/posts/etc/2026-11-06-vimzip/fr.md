+++
title = "vimzip : un environnement Vim qu'il suffit de décompresser"
description = "Une seule archive liée statiquement qui transforme n'importe quelle machine Linux x86-64 en mon environnement Vim complet — sans root, sans gestionnaire de paquets, sans surprise de glibc. Comment c'est construit et pourquoi chaque pièce est là."
tags = ["vim", "linux", "tools"]
updated = 2026-11-09
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
- **fzf.vim**, **nerdtree** (+git), **gitgutter**, **fugitive**, **gv.vim**, **lightline** (+bufferline)

plus les intégrés de Vim qu'on oublie : `comment`, `editorconfig`, `matchit`, `termdebug` et `osc52`.

## Les choix de config qui comptent vraiment

Un binaire portable, c'est la moitié ; l'autre moitié, c'est qu'il se comporte comme un éditeur déjà réglé.

**`vim .` est un projet, pas un fichier.** Démarrer dans un dossier restaure une session par dossier (splits ouverts, dernier fichier touché) et ouvre l'arbre. `Space p` (ou `:Project chemin`) change de projet sur place : il enregistre les onglets de celui-ci, ferme ses terminaux et restaure ceux de l'autre. L'arbre suit le fichier affiché et grise les entrées ignorées par git, comme l'explorateur de VS Code, et `:q` ferme l'onglet courant et ne quitte Vim que sur le dernier. La session et le reste — historique d'annulation, swap, tags — vivent dans `~/.cache/vimzip`, jamais éparpillés dans le projet.

**Sauvegarde auto, parce que j'oublie.** Quitter le mode insertion, perdre le focus ou changer de buffer déclenche un `update` silencieux, et une rafale de modifications en mode normal est enregistrée une seule fois, une seconde après la dernière. Avec l'annulation persistante (`undofile`), ça ressemble plus à un éditeur moderne qu'au Vim classique.

**Presse-papiers par-dessus SSH.** Les copies (`y`) sont renvoyées au presse-papiers système via les séquences OSC 52, donc un `y` sur une machine distante atterrit dans votre presse-papiers *local* si le terminal le gère — sans forwarding X, sans `xclip`.

**Aller à la définition, en douceur.** Avec `clangd`, c'est un vrai saut LSP ; sans lui, vimzip retombe sur les tags du projet, et si même ceux-là manquent, il génère à la volée les tags des *en-têtes système inclus par le fichier courant* (en lisant `cc -M` avec les options d'include du build tirées de `compile_commands.json` s'il existe), pour que `gd` sur `malloc` atterrisse quand même dans l'en-tête de la libc.

**Le blame sur la ligne qu'on lit.** Laissez le curseur immobile et le dernier commit de la ligne (auteur, il y a combien de temps, résumé) s'affiche en grisé après la fin de la ligne, à la manière de GitLens. Un `git blame --porcelain -L n,n` tourne en tâche de fond sur `CursorHold` et le résultat est dessiné comme une propriété de texte Vim : ça ne bloque jamais la saisie et ça disparaît dès que le curseur bouge. `Space gc` ouvre le commit complet dans une popup, et `Space gh` / `Space gl` montrent l'historique du fichier ou tout le graphe des commits via gv.vim. Griser les fichiers ignorés dans l'arbre repose sur une petite astuce : le marqueur git « ignoré » est une espace insécable, qui s'affiche vide mais donne quand même à la règle de syntaxe quelque chose à repérer.

**Complétion depuis tout le projet.** `Ctrl+n` propose aussi les mots des fichiers sources du dépôt, y compris ceux qu'on n'a pas ouverts. ripgrep les rassemble dans un fichier trié en tâche de fond et chaque complétion fait un grep dessus, donc la mémoire reste stable même sur d'énormes projets.

Il y a même un panneau d'agent IA (`Space a`) qui lance le premier trouvé parmi `claude`/`codex`/`gemini`/`aider`/`opencode`. [Les raccourcis complets, avec captures d'écran, font l'objet d'un autre article](/fr/posts/etc/vimzip-keymap/).

## Petit, et rapide sur les gros projets

| | vimzip | VS Code + extension C/C++ |
|---|---|---|
| Téléchargement | 21 Mo | 357 Mo (215 + 142) |
| Installé | 74 Mo | 1 033 Mo (632 + 401) |
| Envoyé sur un serveur SSH | les mêmes 74 Mo | VS Code Server 283 Mo + extensions |
| Prérequis serveur | n'importe quel Linux x86-64 | glibc 2.28 ou plus |

La colonne VS Code correspond à VSCodium 1.143 et cpptools 1.35 compilés à partir des mêmes sources. La release retire aussi ce qui ne sert pas ici, le tutoriel de Vim et les fichiers d'orthographe non UTF-8, et elle est compressée avec `gzip -9`.

Ouvrir un fichier C avec l'arbre, le linting et les marques git actifs prend environ 70 ms et 30 Mo de mémoire. Le test plus dur, c'est LLVM, environ 190 000 fichiers, où la première version ne tenait pas. Chaque correction consistait à travailler moins :

- **N'indexer que les fichiers sources.** Les tags viennent de `rg --files` limité aux langages sources, et au-delà de 30 000 fichiers les dossiers de tests sont ignorés. Les tags de LLVM sont passés de 474 Mo à 140 Mo.
- **Recherche dichotomique dans les tags.** Avec `tagcase=match`, Vim fait une recherche dichotomique dans le fichier de tags trié au lieu de le parcourir : `gd` sur LLVM est passé de 540 ms à 1 ms.
- **Trier l'arbre sur une clé par entrée.** NERDTree triait les dossiers avec un comparateur en Vimscript, `N log N` appels, donc un dossier de 5 500 fichiers prenait 14 secondes. vimzip calcule une clé par entrée et utilise le tri intégré de Vim, et construit l'affichage dans une liste au lieu d'allonger une chaîne.
- **Ne pas bloquer le démarrage.** Les premiers appels git de gitgutter partent après le premier écran, et la liste de mots du projet se construit entièrement en tâche de fond.

Avec tout ça, `vim .` dans LLVM s'ouvre en environ 0,14 s et utilise environ 31 Mo.

## Le garder à jour sans y toucher

Les versions des outils sont épinglées dans un seul `versions.env`. Une GitHub Action hebdomadaire résout les derniers tags amont, et si quelque chose a changé elle reconstruit toute l'archive, lance un test de fumée (démarrer Vim sans interface, vérifier que les plugins sont chargés, générer les tags d'aide) et ouvre une PR. Fusionner la PR publie une nouvelle release. Le truc reste donc à jour tout seul, et chaque release en est une qui a réellement démarré.

## Pourquoi j'aime ça

La philosophie est ennuyeuse exprès : **un fichier, pas d'étape d'installation au-delà du décompressage, pas d'état hors de deux dossiers, et le supprimer ne laisse aucune trace.** Ça ne cherche pas à être un Neovim-avec-tout. C'est la plus petite chose qui fait qu'une machine Linux inconnue ressemble à la mienne, et elle marche pareil que la machine date de 2014 ou d'hier.
