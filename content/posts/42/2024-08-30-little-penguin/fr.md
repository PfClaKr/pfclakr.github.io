+++
title = "little-penguin-1 : dix petits pas dans le noyau Linux"
description = "Compiler et démarrer son propre noyau, écrire des modules, en charger un au branchement d'un clavier USB, un périphérique caractère misc, debugfs avec un mutex, et parcourir l'arbre des montages depuis procfs."
tags = ["42", "linux", "kernel", "c"]
+++

**little-penguin-1** est une série de dix petits exercices, inspirée de l'ancien Eudyptula Challenge,
qui mènent de « je n'ai jamais compilé de noyau » à l'écriture de petits pilotes. Chacun est court.
Ensemble, ils couvrent toute la boucle du travail sur le noyau : compiler, démarrer, écrire un module, le
charger et le déboguer ([source](https://github.com/PfClaKr/Ecole42Paris_LittlePenguin)).

| # | Exercice |
|---|---|
| 00 | compiler le dernier arbre de Linus avec son propre `CONFIG_LOCALVERSION` et démarrer dessus |
| 01 | un module « Hello world » |
| 02 | modifier `EXTRAVERSION` dans le `Makefile` principal, recompiler, rendre le diff comme un patch |
| 03 | corriger un fichier jusqu'à ce que `checkpatch.pl --strict` se taise |
| 04 | charger un module automatiquement au branchement d'un clavier USB |
| 05 | un périphérique caractère misc, `/dev/fortytwo` |
| 06 | compiler et démarrer `linux-next` |
| 07 | un répertoire debugfs avec trois fichiers |
| 08 | nettoyer un module volontairement illisible et comprendre ce qu'il fait |
| 09 | un fichier `/proc/mymounts` qui liste les points de montage |

## Compiler et démarrer

L'exercice 00 demande surtout de la patience : cloner l'arbre, `make menuconfig`, régler
`CONFIG_LOCALVERSION`, compiler, `make modules_install install`, redémarrer et vérifier `uname -r`. La
leçon pratique, c'est l'espace disque. Chaque compilation du noyau, plus ses modules installés sous
`/lib/modules`, prend des gigaoctets. Mes notes conseillent au moins 50 Go pour la VM, et même ça s'est
rempli à l'exercice 02, jusqu'à ce que je supprime les modules des anciens noyaux.

## Un module, c'est deux fonctions

```c
static int __init hello_init(void)
{
    printk(KERN_INFO "Hello world!\n");
    return 0;
}

static void __exit hello_exit(void)
{
    printk(KERN_INFO "Cleaning up module.\n");
}

module_init(hello_init);
module_exit(hello_exit);
```

`printk` n'écrit pas dans un terminal. Il écrit dans le tampon circulaire des logs
du noyau, qu'on lit avec `dmesg`. `KERN_INFO` n'est pas du tout un argument : c'est une chaîne
littérale (`"\001" "6"`) que le compilateur colle devant la chaîne de format, et le niveau de log est lu
dans ce préfixe.

## Le branchement à chaud avec udev

L'exercice 04 veut que le module se charge au branchement d'un clavier USB. Le noyau annonce le nouveau
périphérique, et **udev**, en espace utilisateur, le compare à des règles. Un clavier USB est
l'interface de classe 3 (HID), protocole 1 (clavier) :

```text
ACTION=="add", SUBSYSTEM=="usb", ATTR{bInterfaceClass}=="03", ATTR{bInterfaceProtocol}=="01", \
    RUN+="/sbin/insmod /root/main.ko"
```

La façon « propre » côté noyau est un `MODULE_DEVICE_TABLE(usb, …)` pour que `modprobe` charge le
module à partir du modalias du périphérique, mais la règle udev rend le mécanisme visible.

## Un périphérique caractère misc

`/dev/fortytwo` doit renvoyer mon login en lecture, et n'accepter une écriture que si c'est exactement
mon login, sinon échouer avec `EINVAL`. Un périphérique misc reçoit un numéro mineur dynamique sous le
majeur misc partagé, donc l'enregistrement tient en un appel. Le travail est dans les
`file_operations` :

```c
static ssize_t ft_read(struct file *f, char __user *buf, size_t count, loff_t *ppos)
{
    return simple_read_from_buffer(buf, count, ppos, LOGIN, strlen(LOGIN));
}

static ssize_t ft_write(struct file *f, const char __user *buf, size_t count, loff_t *ppos)
{
    char kbuf[32];
    ssize_t len = simple_write_to_buffer(kbuf, sizeof(kbuf) - 1, ppos, buf, count);

    if (len < 0)
        return len;
    kbuf[len] = '\0';
    return strcmp(kbuf, LOGIN) == 0 ? count : -EINVAL;
}
```

Le noyau ne peut jamais déréférencer directement un pointeur de l'espace utilisateur (`__user` les
marque pour l'analyse statique). Les fonctions `simple_*_buffer` enveloppent
`copy_to_user`/`copy_from_user` et gèrent la position dans le fichier.

Cette position m'a piégé pendant les tests. `read` et `write` sur le **même** descripteur partagent
`*ppos`. Après avoir écrit 5 octets, un `read` commence à la position 5, après la fin du login, et ne
renvoie rien. Fermer et rouvrir, ou `lseek(fd, 0, SEEK_SET)`, la remet à zéro. Tout fichier ordinaire se
comporte ainsi, mais on l'oublie facilement quand c'est soi qui implémente le fichier.

## debugfs et un mutex

L'exercice 07 crée `/sys/kernel/debug/fortytwo/` avec trois fichiers : `id` (le même comportement que
`/dev/fortytwo`), `jiffies` (en lecture seule, le compteur de ticks du noyau, exposé en `u32` ou `u64` selon la taille
de mot de l'architecture) et `foo` (une page de
stockage que root peut écrire et que tout le monde peut lire). Comme deux processus peuvent écrire
`foo` en même temps, l'accès passe par un mutex :

```c
static char foo_page[PAGE_SIZE];
static size_t foo_page_len;
static DEFINE_MUTEX(foo_mutex);

static ssize_t foo_write(struct file *f, const char __user *buf, size_t count, loff_t *ppos)
{
    ssize_t ret;

    if (count > PAGE_SIZE)
        return -EINVAL;
    mutex_lock(&foo_mutex);
    ret = simple_write_to_buffer(foo_page, PAGE_SIZE, ppos, buf, count);
    foo_page_len = *ppos;
    mutex_unlock(&foo_mutex);
    return ret;
}
```

Un mutex convient ici parce qu'on est en contexte de processus et qu'on a le droit de dormir. Dans un
gestionnaire d'interruption, il faudrait un spinlock.

## Parcourir l'arbre des montages

L'exercice 09 liste chaque point de montage dans `/proc/mymounts`, comme un `/proc/mounts` simplifié. Les
fichiers procfs sont générés à la lecture, donc le module utilise l'interface `seq_file` : `single_open`
plus un callback `show` qui écrit dans un tampon géré par le noyau.

Trouver les montages, c'est là que la version du noyau compte. Les anciens tutoriels parcourent
`mnt_ns->list`, une liste chaînée. Dans les noyaux récents, les montages d'un namespace vivent dans un
**arbre rouge-noir**, `mnt_ns->mounts`, donc le module le parcourt avec `rb_entry` :

```c
static void iter_mount_tree(struct seq_file *m, struct rb_node *node)
{
    if (!node)
        return;
    show_mount(m, rb_entry(node, struct mount, mnt_node));
    iter_mount_tree(m, node->rb_left);
    iter_mount_tree(m, node->rb_right);
}
```

Deux réserves honnêtes : `struct mount` est privée (elle vient de `fs/mount.h`, pas d'un en-tête public),
donc le module ne compile qu'avec un arbre source complet du noyau. Et le parcours ne prend aucun verrou :
un montage ou démontage simultané pourrait modifier l'arbre pendant la lecture. Le vrai `/proc/mounts`
tient le verrou du namespace pendant l'itération.

## Ce qui reste

L'essentiel de little-penguin, c'est apprendre où vivent les choses : les logs dans `dmesg`, les
périphériques dans `/dev` gérés par udev, les réglages de débogage dans debugfs, les vues des processus
dans procfs. Une fois cette carte connue, le code noyau intimide beaucoup moins, et c'est ce qui a rendu
abordable l'écriture d'un noyau de zéro dans [KFS](/fr/posts/42/kfs-rust-kernel/).
