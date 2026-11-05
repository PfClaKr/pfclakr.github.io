+++
title = "ft_shield : comment une porte dérobée se cache, et comment la trouver"
description = "Un trojan éducatif pour une VM jetable, lu du côté du défenseur : à quoi ressemble vraiment la persistance via systemd, sur le disque et dans l'état des processus, et les endroits exacts qu'un intervenant vérifie pour l'attraper."
tags = ["42", "security", "linux"]
+++

**ft_shield** est un projet de sécurité de 42 : construire un petit « trojan » et ne l'exécuter **que dans
une machine virtuelle jetable**. Le but n'est d'attaquer personne. C'est de voir, de l'intérieur, les
quelques choses que toute porte dérobée persistante doit faire, pour les reconnaître instantanément quand
c'est vous qui défendez une machine ([source](https://github.com/PfClaKr/Ecole42Paris_ft_shield)). Le dépôt
commence par cet avertissement, fournit un `reset.sh` pour supprimer le programme, et cet article reste du
côté du défenseur : quoi chercher, et où.

> ⚠️ Un programme qui ouvre un shell distant est dangereux par définition. ft_shield a sa place dans une VM
> isolée et jetable, et nulle part ailleurs. Tout ce qui suit est présenté comme de la détection, parce que
> c'est la compétence qui se transfère.

## Ce que toute porte dérobée persistante doit faire

En retirant les détails, une porte dérobée n'a que quatre tâches. Chacune laisse une trace.

1. **Survivre à un redémarrage** (persistance) — s'enregistrer là où le système lit au démarrage.
2. **Tourner sans surveillance** (daemonize) — se détacher de tout terminal et continuer en arrière-plan.
3. **Attendre l'opérateur** (un canal) — en général un port réseau, protégé par un secret.
4. **Rester installée** (résilience) — résister à un arrêt ou une suppression désinvolte.

ft_shield fait les trois premières de la façon la plus ordinaire possible, et c'est exactement la leçon : la
persistance moderne n'a rien d'exotique. Elle ressemble à un service normal.

## Une persistance qui a l'air légitime

ft_shield s'installe comme un **service systemd** : il écrit un fichier d'unité, copie son binaire dans un
chemin système (`/bin/ft_shield`), et demande à systemd de l'activer. Dès lors, l'init le démarre à chaque
boot, gratuitement, sans ruse. Il détecte aussi s'il a été lancé par l'init (en vérifiant si son processus
parent est `systemd`, le PID 1) pour ne s'installer qu'une fois.

Pour un défenseur, c'est le premier et le meilleur endroit où regarder. Un service malveillant est visible
dans tous les endroits habituels, si on sait qu'ils existent :

```bash
systemctl list-unit-files --state=enabled      # quelque chose d'inconnu ici ?
ls -lt /etc/systemd/system/ /lib/systemd/system # fichiers d'unité les plus récents d'abord
systemctl cat <service-suspect>                 # lire son chemin ExecStart
```

Les indices : une unité dont le nom imite un vrai démon, un `ExecStart` pointant vers un chemin bizarre
(`/bin`, `/tmp`, un répertoire personnel), et une date de création qui colle à un incident plutôt qu'à une
installation. Un service légitime a un paquet derrière lui ; `dpkg -S /bin/ft_shield` (ou `rpm -qf`) qui
répond « n'appartient à aucun paquet » est un signal bruyant.

## La daemonisation, et ce qu'elle laisse

Pour tourner sans surveillance, une porte dérobée se détache du terminal qui l'a lancée : elle fork, le
parent sort, l'enfant continue en arrière-plan. Sous systemd, le gestionnaire de services le fait pour vous,
plus propre pour l'attaquant et toujours visible pour le défenseur. Un processus en arrière-plan sans
terminal de contrôle se voit clairement :

```bash
ps -eo pid,ppid,user,tty,cmd | grep '?'    # TTY à '?' = pas de terminal
ss -lntp                                   # quel processus possède chaque port en écoute
lsof -p <pid>                              # ses fichiers et sockets ouverts
```

Le processus tourne en **root**, re-parenté au PID 1, en écoute sur un port. Rien de tout ça n'est caché par
le programme ; c'est juste facile à manquer si on ne regarde pas régulièrement.

## Le canal : un port en écoute derrière un secret

ft_shield écoute sur le TCP **4242** et, avant de donner un shell, demande un `Keycode`. Le secret empêche un
scanner de ports au hasard de tomber dessus, mais il ne cache en rien le **port ouvert lui-même**. C'est
l'angle de détection : pas besoin du mot de passe pour remarquer la porte.

```bash
ss -lntp | grep 4242        # un écouteur inattendu
# depuis une autre machine, prudemment, en labo seulement :
nmap <ip-vm>                # un port ouvert inexpliqué est le constat
```

Un socket en écoute inexpliqué sur un serveur est l'un des indicateurs les plus parlants qui soient. Un vrai
service correspond à un paquet connu et à un port documenté ; le port d'une porte dérobée ne correspond à
rien.

## La résilience, et ses limites

ft_shield essaie de rester installé, mais comme le montre le `reset.sh` du projet, une persistance basée sur
systemd est aussi une *suppression* basée sur systemd. Le démontage est exactement l'inverse de
l'installation :

```bash
systemctl stop ft_shield && systemctl disable ft_shield
rm -f /etc/systemd/system/ft_shield.service /bin/ft_shield
systemctl daemon-reload
```

La leçon plus profonde, c'est que persistance et détection utilisent la même poignée d'interfaces système.
Tout ce qu'un attaquant peut enregistrer, un défenseur peut l'énumérer. La seule asymétrie, c'est
l'attention : l'attaquant n'a besoin que d'un endroit, le défenseur doit tous les connaître.

## La check-list du défenseur

Ce que ft_shield enseigne vraiment, c'est une courte liste d'endroits à balayer sur un hôte Linux suspect :

- **Persistance au boot :** les unités systemd (`/etc/systemd/system`, `/lib/systemd/system`, unités
  utilisateur), plus les classiques — cron (`/etc/cron*`, `crontab -l`), `rc.local`, scripts d'init,
  fichiers rc du shell, `~/.ssh/authorized_keys`.
- **Processus :** tout ce qui tourne en root sans TTY, re-parenté au PID 1, ou dont le nom imite un vrai
  démon sans paquet derrière.
- **Réseau :** chaque socket en écoute (`ss -lntp`) rapproché des services attendus.
- **Fichiers :** des binaires dans des chemins système n'appartenant à aucun paquet, et des fichiers dont
  les horodatages se regroupent autour d'un moment suspect.

Avoir construit la chose qui se cache dans chacun de ces endroits rend le balayage instinctif. La meilleure
façon d'apprendre où vit une porte dérobée, c'est, une fois, dans une VM qu'on s'apprête à supprimer, d'en
poser une soi-même.
