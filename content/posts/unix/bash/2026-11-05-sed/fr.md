+++
title = "sed : l'éditeur de flux, au-delà de s///"
description = "Comment sed fonctionne vraiment — pattern space, adresses, hold space — et les one-liners qui justifient de l'apprendre : plages, édition sur place, lignes multiples, et les pièges GNU vs BSD."
tags = ["bash", "sed", "unix", "shell"]
+++

La plupart des gens connaissent exactement une commande sed : `s/ancien/nouveau/g`. Ça couvre
beaucoup de cas, mais sed est un petit langage de programmation, et c'est le reste qui rend
possibles des one-liners comme « changer `port` uniquement dans la section `[server]` » ou
« recoller les lignes qui finissent par un antislash ». Cet article commence par le modèle, parce
qu'une fois qu'il est compris, les scripts cryptiques cessent de l'être.

Tous les exemples ont été exécutés avec GNU sed 4.9. Les différences avec sed BSD/macOS sont signalées.

## Comment sed pense : une ligne à la fois

sed lit son entrée ligne par ligne dans un tampon appelé **pattern space**, exécute tout le script
dessus, affiche le pattern space, puis passe à la ligne suivante. C'est toute la boucle.

```bash
printf 'a\nb\nc\n' | sed 's/b/B/'
# a
# B
# c
```

L'affichage automatique à la fin de chaque cycle explique l'option `-n` : elle le désactive, et
seules les lignes affichées explicitement avec `p` sortent.

```bash
printf 'a\nb\nc\n' | sed -n '2p'     # b
sed -n '5,10p' gros.log             # lignes 5 à 10, comme head | tail
sed 3q fichier                      # 3 lignes puis quitter : head -n 3
```

## La substitution, correctement

La forme complète est `s/regex/remplacement/drapeaux`.

- **Drapeaux** : `g` remplace toutes les occurrences de la ligne, un nombre ne remplace que la
  N-ième, `p` affiche la ligne si une substitution a eu lieu (utile avec `-n`), et GNU ajoute `I`
  pour ignorer la casse.
- **N'importe quel délimiteur** : le caractère après `s` sert de délimiteur, inutile d'échapper les chemins.
- **`&`** dans le remplacement désigne toute la correspondance ; **`\1`…`\9`** les groupes capturés.
- **`-E`** active les expressions régulières étendues (`+`, `?`, `()`, `|` sans antislash).
  GNU et BSD acceptent tous deux `-E`.

```bash
echo 'one one one' | sed 's/one/two/2'                 # one two one
echo 'PATH=/usr/local/bin' | sed 's#/usr/local#/opt#'  # PATH=/opt/bin
echo 'error 42' | sed -E 's/[0-9]+/[&]/'               # error [42]
echo 'Turing, Alan' | sed -E 's/^([^,]+), (.+)$/\2 \1/' # Alan Turing
```

GNU sed comprend aussi la conversion de casse dans le remplacement : `\U` met en majuscules jusqu'à
`\E`, `\L` en minuscules, `\u`/`\l` agissent sur un seul caractère. Pratique pour renommer des identifiants :

```bash
echo 'max_retry_count' | sed -E 's/_([a-z])/\U\1/g'    # maxRetryCount (GNU uniquement)
```

## Les adresses : choisir les lignes

Une commande peut être précédée d'une **adresse** qui décide sur quelles lignes elle s'applique.
Sans adresse, elle s'applique à toutes.

| Adresse | Signification |
|---|---|
| `3` | la ligne 3 |
| `$` | la dernière ligne |
| `/re/` | les lignes qui correspondent à `re` |
| `10,20` | les lignes 10 à 20 |
| `/BEGIN/,/END/` | d'une ligne contenant `BEGIN` à la prochaine contenant `END` |
| `/re/,+2` | la ligne trouvée et les deux suivantes (GNU) |
| `0~2` | une ligne sur deux : premier~pas (GNU) |
| `addr!` | toutes les lignes qui **ne** correspondent **pas** à `addr` |

C'est avec les plages que sed devient vraiment utile pour les fichiers de configuration. Changer
`port` uniquement dans la section `[server]` d'un fichier INI, sans toucher à `[client]` :

```bash
cat config.ini
# [server]
# port=80
# [client]
# port=1

sed '/^\[server\]/,/^\[/ s/^port=.*/port=8080/' config.ini
# [server]
# port=8080
# [client]
# port=1
```

La plage commence à `[server]` et se termine à la prochaine ligne qui commence par `[` : la
substitution ne voit que les lignes entre les deux. Quelques autres :

```bash
sed -e '/^[[:space:]]*#/d' -e '/^[[:space:]]*$/d' app.conf  # retirer commentaires et lignes vides
sed -n '/ERROR/,+2p' app.log                                # chaque erreur et deux lignes de contexte
sed '/keep/!d' liste.txt                                    # supprimer tout ce qui ne correspond pas
seq 1 6 | sed -n '0~2p'                                     # 2 4 6
```

## Les autres commandes que s

| Commande | Effet |
|---|---|
| `d` | supprime le pattern space et passe au cycle suivant |
| `p` | affiche le pattern space |
| `q` | affiche (sauf `-n`) et quitte |
| `a texte`, `i texte`, `c texte` | ajoute après, insère avant, ou remplace la ligne |
| `y/abc/xyz/` | translittère des caractères, comme `tr` |
| `=` | affiche le numéro de ligne courant |
| `r fichier`, `w fichier` | insère un fichier dans la sortie, écrit la ligne dans un fichier |

```bash
sed '/^\[server\]/a timeout=30' config.ini   # ajouter une ligne après [server]
printf 'a\nold\nb\n' | sed '/old/c\
new'                                         # remplacer une ligne entière
sed -n '$=' fichier                          # compter les lignes, comme wc -l
```

La forme `a texte` sur une seule ligne est une extension GNU. POSIX (et BSD) exigent un antislash
et un saut de ligne : `a\` puis le texte à la ligne suivante, d'où l'allure étrange des scripts
écrits pour macOS.

## L'édition sur place, et son piège de portabilité

`-i` réécrit le fichier au lieu d'afficher sur la sortie standard. Le problème : GNU et BSD ne sont
pas d'accord sur la syntaxe.

```bash
sed -i 's/foo/bar/' fichier        # GNU
sed -i '' 's/foo/bar/' fichier     # BSD / macOS : le suffixe de sauvegarde est obligatoire, '' = aucun
sed -i.bak 's/foo/bar/' fichier && rm fichier.bak   # fonctionne partout
```

:::warning
Deux autres choses à savoir sur `-i` : il écrit un nouveau fichier puis le renomme par-dessus
l'ancien, donc les liens physiques vers l'original sont rompus (et un lien symbolique est remplacé
par un fichier normal, sauf avec l'option GNU `--follow-symlinks`). Et il réécrit le fichier même
si rien ne correspond, ce qui change sa date de modification.
:::

## Le hold space : les astuces sur plusieurs lignes

En plus du pattern space, sed dispose d'un second tampon, le **hold space**, qui persiste d'une
ligne à l'autre. Cinq commandes échangent des données entre les deux :

| Commande | Effet |
|---|---|
| `h` / `H` | copie / ajoute le pattern space dans le hold space |
| `g` / `G` | copie / ajoute le hold space dans le pattern space |
| `x` | échange les deux |

Avec `N` (ajouter la ligne d'entrée suivante au pattern space), les étiquettes (`:a`) et les
branchements (`b a` toujours, `t a` seulement si une substitution a réussi), on couvre la plupart
des traitements sur plusieurs lignes.

**Inverser un fichier** (ce que fait `tac`) :

```bash
printf '1\n2\n3\n' | sed -n '1!G;h;$p'
# 3
# 2
# 1
```

Ligne par ligne : sauf sur la première, on ajoute ce qu'on a gardé jusque-là (`G`) ; on stocke le
résultat (`h`) ; sur la dernière ligne, on l'affiche. Le hold space grandit sous la forme « ligne
courante, puis tout ce qui précède ».

**Joindre toutes les lignes avec une virgule :**

```bash
printf 'a\nb\nc\n' | sed ':a;N;$!ba;s/\n/,/g'      # a,b,c
```

`N` ajoute sans cesse des lignes au pattern space et `$!ba` boucle jusqu'à la dernière ; une seule
substitution remplace ensuite les sauts de ligne. (`paste -sd, fichier` fait la même chose, plus lisiblement.)

**Recoller les lignes qui finissent par un antislash**, comme dans un script shell ou un Makefile :

```bash
printf 'one \\\ntwo \\\nthree\nfour\n' | sed -e ':a' -e '/\\$/N; s/\\\n//; ta'
# one two three
# four
```

**Afficher la ligne qui précède chaque correspondance :**

```bash
printf 'a\nb\nMATCH\nc\n' | sed -n '/MATCH/{x;p;x;}; h'    # b
```

Chaque ligne est mémorisée avec `h` ; quand une ligne correspond, on échange pour récupérer la
précédente, on l'affiche, puis on échange à nouveau.

## GNU vs BSD en un coup d'œil

| | GNU sed | sed BSD / macOS |
|---|---|---|
| Édition sur place | `-i` | `-i ''` |
| `\n`, `\t` dans le remplacement | oui | non |
| Conversion de casse `\U`, `\L` | oui | non |
| `premier~pas`, `addr,+N` | oui | non |
| `a texte` sur une ligne | oui | non, utiliser `a\` + saut de ligne |
| `-E` | oui | oui |

Sur macOS, `brew install gnu-sed` fournit `gsed`. Si un script doit tourner partout, restez dans le
sous-ensemble POSIX et testez-le avec `sed --posix` sous GNU.

## Quand ne pas utiliser sed

sed travaille par lignes et ne connaît ni les champs ni l'imbrication. Quand il s'agit de « la
troisième colonne quand la deuxième dépasse 100 », prenez awk. Pour du JSON, `jq` ; pour du CSV avec
des virgules entre guillemets, un vrai parseur ; et si le script multi-ligne demande plus de deux ou
trois astuces `N`/`G`, quelques lignes d'awk ou de Python seront plus lisibles dans six mois.
