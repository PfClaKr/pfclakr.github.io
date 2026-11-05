+++
title = "awk, xargs et find : les commandes qui mettent du temps à faire tilt"
description = "Les outils de pipeline qu'on copie depuis Stack Overflow sans vraiment les comprendre : le modèle motif–action d'awk, xargs et les noms de fichiers avec espaces, find -exec avec ; ou +, et les petits utilitaires tr, cut, comm, paste et column."
tags = ["bash", "awk", "unix", "shell"]
+++

L'[article sur sed](/fr/posts/unix/bash/sed/) se terminait par « utilisez awk quand il s'agit de
champs ». Celui-ci reprend à partir de là, et ajoute les autres commandes présentes dans tout
pipeline sérieux mais rarement expliquées : `xargs`, `find -exec`, et une poignée de petits outils
qui remplacent chacun une boucle qu'on écrirait sinon à la main.

Les exemples ont été exécutés avec GNU coreutils/findutils et `mawk` ; ce qui est propre à une
implémentation est signalé.

## awk : motifs et actions

Un programme awk est une liste de règles `motif { action }`. Pour chaque ligne d'entrée (un
**enregistrement**), awk la découpe en **champs** `$1`, `$2`, … `$NF`, puis exécute chaque action dont
le motif correspond. Un motif absent correspond à toutes les lignes ; une action absente affiche la ligne.

```bash
awk '{ print $1 }' fichier                   # premier champ de chaque ligne
awk -F: '$3 >= 1000 { print $1, $7 }' /etc/passwd
# alice /bin/zsh
# bob /bin/bash
```

`-F` définit le séparateur de champs en entrée (les blancs par défaut). Quelques variables intégrées
font l'essentiel du travail :

| Variable | Signification |
|---|---|
| `NR` | numéro de l'enregistrement courant, tous fichiers confondus |
| `FNR` | numéro de l'enregistrement dans le fichier courant |
| `NF` | nombre de champs de l'enregistrement (`$NF` est le dernier champ) |
| `FS` / `OFS` | séparateur de champs en entrée / en sortie |
| `BEGIN` / `END` | motifs spéciaux exécutés avant la première et après la dernière ligne |

```bash
awk -F: -v OFS='\t' '{ print $1, $6 }' /etc/passwd   # utilisateur<TAB>répertoire
awk '{ s += $NF } END { print s }' tailles.txt       # somme de la dernière colonne
awk -F: '{ printf "%-8s %5d\n", $1, $3 }' /etc/passwd # colonnes alignées
```

### Tableaux associatifs : un « group by » en une ligne

Les tableaux d'awk sont des tables de hachage indexées par des chaînes, ce qui transforme un
« group by » en one-liner. Dans le format de log courant, l'IP du client est `$1`, le chemin `$7`, le
code de statut `$9` et la taille `$10` :

```bash
# 203.0.113.7 - - [05/Nov/2026:10:12:01 +0100] "GET /posts/ HTTP/1.1" 200 5120 "-" "curl/8.5"

awk '$9 == 404 { print $7 }' access.log                   # chemins qui ont renvoyé 404
awk '{ bytes[$1] += $10 } END { for (ip in bytes) print bytes[ip], ip }' access.log | sort -rn
```

Deux idiomes méritent d'être retenus, parce qu'on les croise partout.

**Supprimer les doublons sans trier** (garde la première occurrence, dans l'ordre) :

```bash
printf 'b\na\nb\nc\na\n' | awk '!seen[$0]++'
# b
# a
# c
```

`seen[$0]++` renvoie l'ancien compteur, qui vaut 0 (faux) la première fois qu'une ligne apparaît ;
`!` le rend vrai, et un motif sans action affiche la ligne.

**Filtrer un fichier par les clés d'un autre :**

```bash
awk -F: 'NR == FNR { ok[$1]; next } $1 in ok' utilisateurs-autorises.txt /etc/passwd
```

Pendant la lecture du premier fichier, `NR` (global) est égal à `FNR` (par fichier) : la première
règle mémorise chaque clé et `next` saute le reste. Pour le second fichier ce n'est plus vrai, et
`$1 in ok` ne garde que les lignes dont la clé a été vue. C'est une jointure, sans trier aucune des entrées.

## xargs : transformer des lignes en arguments

Beaucoup de commandes prennent des noms de fichiers en arguments, pas sur l'entrée standard. La
solution tentante, `rm $(find . -name '*.log')`, casse dès qu'un nom contient une espace, et peut
dépasser la longueur maximale de la ligne de commande sur une grosse arborescence :

```bash
for f in $(find files -name '*.log'); do echo "[$f]"; done
# [files/a]
# [b.log]        ← « a b.log » coupé en deux
# [files/c.log]
```

`xargs` lit des éléments sur l'entrée standard et lance une commande avec eux comme arguments, par
lots, pour que la ligne de commande ne soit jamais trop longue. Pour être sûr avec n'importe quel nom,
on sépare les éléments par un octet NUL des deux côtés :

```bash
find files -name '*.log' -print0 | xargs -0 rm --
```

| Option | Effet |
|---|---|
| `-0` | éléments séparés par NUL (avec `find -print0`, `grep -lZ`, …) |
| `-n N` | au plus N arguments par commande |
| `-P N` | jusqu'à N commandes en parallèle |
| `-I{}` | une commande par ligne d'entrée, `{}` remplacé par la ligne |
| `-r` | GNU : ne pas lancer la commande du tout si l'entrée est vide |

```bash
echo 1 2 3 4 | xargs -n 2 echo           # « 1 2 » puis « 3 4 »
printf 'x\ny\n' | xargs -I{} echo "item: {}"
find . -name '*.png' -print0 | xargs -0 -P 4 -n 16 optipng   # 4 processus, 16 fichiers chacun
```

La ligne `-r` compte plus qu'il n'y paraît : GNU xargs lance la commande une fois même sans entrée,
donc `... | xargs rm` avec une liste vide exécute un `rm` nu. Ajoutez `-r`, ou utilisez `find -exec`.

## find -exec : `\;` ou `+`

`find` peut lancer les commandes lui-même, ce qui évite complètement le problème des guillemets :

```bash
find files -name '*.log' -exec echo {} \;    # un echo par fichier
find files -name '*.log' -exec echo {} +     # un seul echo avec tous les fichiers, comme xargs
```

`\;` démarre un processus par fichier ; `+` regroupe autant de fichiers que possible par appel, ce qui
est bien plus rapide sur de grosses arborescences. N'utilisez `\;` que si la commande ne prend qu'un fichier.

Les tests se combinent par un ET implicite, `-o` pour le OU, et des parenthèses échappées pour grouper.
`-prune` ignore tout un répertoire :

```bash
find files \( -name '*.log' -o -name '*.txt' \) -type f
find proj -path proj/node_modules -prune -o -name '*.js' -print
find /var/log -type f -mtime -1 -size +100M          # modifiés depuis un jour, plus de 100 Mio
find . -name '*.tmp' -delete                         # suppression intégrée, pas besoin de xargs
```

La forme `-prune -o … -print` paraît étrange jusqu'à ce qu'on la lise ainsi : « si le chemin est
`node_modules`, l'élaguer (et ne rien afficher) ; sinon, si c'est un fichier `.js`, l'afficher ». Sans
le `-print` explicite, le répertoire élagué serait lui aussi affiché.

## Les petits outils qui remplacent une boucle

Chacun fait une seule chose, et chacun évite régulièrement d'écrire une boucle `while read`.

**`tr`** remplace ou supprime des caractères :

```bash
echo 'Hello   World' | tr -s ' ' | tr 'a-z' 'A-Z'   # HELLO WORLD
tr -d '\r' < windows.txt > unix.txt                 # retirer les fins de ligne CRLF
```

**`cut`** extrait des colonnes par délimiteur ou par position :

```bash
cut -d: -f1,7 /etc/passwd     # utilisateur:shell
cut -c1-8 fichier             # 8 premiers caractères de chaque ligne
```

**`comm`** compare deux fichiers **triés** ligne par ligne, sur trois colonnes : seulement dans le
premier, seulement dans le second, dans les deux. On masque des colonnes avec `-1`, `-2`, `-3`. Avec
la substitution de processus `<(…)`, aucun fichier temporaire n'est nécessaire :

```bash
comm -12 <(sort a.txt) <(sort b.txt)     # lignes présentes dans les deux
comm -23 <(sort a.txt) <(sort b.txt)     # lignes seulement dans a.txt
```

**`paste`** fusionne des lignes : côte à côte, ou toutes les lignes d'un fichier en une seule :

```bash
paste noms.txt tailles.txt    # nom<TAB>taille
paste -sd, liste.txt          # a,b,c
```

**`sort -k`**, **`uniq -c`**, **`column -t`** terminent la plupart des rapports :

```bash
sort -t, -k2,2n data.csv      # tri numérique sur le 2e champ séparé par des virgules
sort | uniq -c | sort -rn     # compter les occurrences, les plus fréquentes d'abord
column -t rapport.txt         # aligner des colonnes séparées par des blancs
```

`-k2,2n` signifie « la clé commence et finit au champ 2, comparer comme des nombres » ; un simple
`-k2` comparerait du champ 2 jusqu'à la fin de la ligne.

**`tee`** écrit son entrée dans un fichier et sur la sortie standard. C'est aussi la façon standard
d'écrire dans un fichier appartenant à root depuis un pipeline, car dans `sudo cmd > fichier` la
redirection est faite par votre shell, pas par `sudo` :

```bash
echo 'vm.swappiness=10' | sudo tee -a /etc/sysctl.d/99-local.conf > /dev/null
```

## Tout assembler

```bash
# Les dix IP clientes les plus actives
awk '{ print $1 }' access.log | sort | uniq -c | sort -rn | head

# Fichiers source modifiés depuis un jour qui contiennent encore TODO
find src -type f -mtime -1 -print0 | xargs -0 grep -l TODO

# Utilisateurs présents dans /etc/passwd mais absents d'une liste autorisée
comm -23 <(cut -d: -f1 /etc/passwd | sort) <(sort utilisateurs-autorises.txt)
```

Le schéma est toujours le même : un outil sélectionne, un autre transforme, un dernier agrège. Une
fois chacun compris séparément, les longs pipelines cessent de ressembler à de la magie et se lisent
comme une phrase.
