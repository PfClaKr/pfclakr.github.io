+++
title = "little-penguin-1: 리눅스 커널로 들어가는 작은 열 걸음"
description = "직접 커널 빌드하고 부팅하기, 모듈 작성, USB 키보드를 꽂으면 모듈 로드하기, misc 문자 디바이스, 뮤텍스를 쓴 debugfs, procfs에서 마운트 트리 순회하기."
tags = ["42", "linux", "kernel", "c"]
+++

**little-penguin-1**은 예전 Eudyptula Challenge에서 영감을 받은 짧은 과제 열 개로, "커널을 컴파일해 본
적이 없다"에서 작은 드라이버를 쓰는 데까지 데려갑니다. 하나하나는 작지만, 모아 놓으면 커널 작업의 한
바퀴를 모두 돕니다. 빌드하고, 부팅하고, 모듈을 쓰고, 로드하고, 디버깅합니다
([소스](https://github.com/PfClaKr/Ecole42Paris_LittlePenguin)).

| # | 과제 |
|---|---|
| 00 | 리누스의 최신 트리를 내 `CONFIG_LOCALVERSION`으로 빌드하고 부팅 |
| 01 | "Hello world" 모듈 |
| 02 | 최상위 `Makefile`의 `EXTRAVERSION`을 고쳐 다시 빌드하고, diff를 패치로 제출 |
| 03 | `checkpatch.pl --strict`가 조용해질 때까지 파일 고치기 |
| 04 | USB 키보드를 꽂으면 모듈이 자동으로 로드되게 하기 |
| 05 | misc 문자 디바이스 `/dev/fortytwo` |
| 06 | `linux-next` 빌드하고 부팅 |
| 07 | 파일 세 개가 든 debugfs 디렉터리 |
| 08 | 일부러 엉망으로 짠 모듈을 정리하고 무슨 일을 하는지 알아내기 |
| 09 | 마운트 지점을 나열하는 `/proc/mymounts` 파일 |

## 빌드와 부팅

과제 00은 대부분 인내심입니다. 트리를 클론하고, `make menuconfig`, `CONFIG_LOCALVERSION` 설정, 빌드,
`make modules_install install`, 재부팅, `uname -r` 확인. 실용적인 교훈은 디스크 용량입니다. 커널을 한 번
빌드할 때마다, 그리고 `/lib/modules`에 설치된 모듈까지 기가바이트 단위로 차지합니다. 제 노트에는 VM 디스크를
50 GB는 잡으라고 적어 뒀는데, 그것도 과제 02에서 다 차서 옛 커널 모듈을 지워야 했습니다.

## 모듈은 함수 두 개

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

`printk`는 터미널에 쓰지 않습니다. `dmesg`로 읽는 커널 로그 링 버퍼에 씁니다.
`KERN_INFO`는 함수 인자가 아니라, 컴파일러가 형식 문자열 앞에 붙이는 문자열 리터럴(`"\001" "6"`)이고, 로그
레벨은 이 접두사에서 읽어 냅니다.

## udev로 핫플러그

과제 04는 USB 키보드를 꽂으면 모듈이 로드되기를 원합니다. 커널이 새 장치를 알리면 사용자 공간의
**udev**가 규칙과 맞춰 봅니다. USB 키보드는 인터페이스 클래스 3(HID), 프로토콜 1(키보드)입니다.

```text
ACTION=="add", SUBSYSTEM=="usb", ATTR{bInterfaceClass}=="03", ATTR{bInterfaceProtocol}=="01", \
    RUN+="/sbin/insmod /root/main.ko"
```

커널다운 "정석"은 `MODULE_DEVICE_TABLE(usb, …)`을 둬서 `modprobe`가 장치의 modalias로 모듈을 로드하게
하는 것이지만, udev 규칙을 쓰면 동작 원리가 눈에 보입니다.

## misc 문자 디바이스

`/dev/fortytwo`는 읽으면 제 로그인을 돌려주고, 쓰기는 정확히 제 로그인일 때만 받고 아니면 `EINVAL`로
실패해야 합니다. misc 디바이스는 공용 misc 메이저 번호 아래에서 마이너 번호를 동적으로 받으니 등록은 호출
하나면 됩니다. 일은 `file_operations`에 있습니다.

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

커널은 사용자 공간 포인터를 절대 직접 역참조하면 안 됩니다(`__user`는 정적 분석용 표시). `simple_*_buffer`
도우미가 `copy_to_user`/`copy_from_user`를 감싸고 파일 오프셋도 처리해 줍니다.

테스트하다가 이 오프셋에 걸렸습니다. **같은** 파일 디스크립터로 `read`와 `write`를 하면 `*ppos`를 공유합니다.
5바이트를 쓰고 나면 `read`는 오프셋 5, 즉 로그인 끝을 지난 곳에서 시작해서 아무것도 돌려주지 않습니다.
닫았다 다시 열거나 `lseek(fd, 0, SEEK_SET)`로 되돌려야 합니다. 일반 파일도 모두 이렇게 동작하지만, 파일을
직접 구현하는 입장이 되면 잊기 쉽습니다.

## debugfs와 뮤텍스

과제 07은 `/sys/kernel/debug/fortytwo/`에 파일 세 개를 만듭니다. `id`(`/dev/fortytwo`와 같은 동작),
`jiffies`(읽기 전용, 커널 틱 카운터. 아키텍처의 워드 크기에 따라 `u32`나 `u64`로 노출), `foo`(root가 쓰고 누구나 읽을 수 있는 한 페이지짜리 저장소). 두
프로세스가 동시에 `foo`에 쓸 수 있으니 뮤텍스로 접근합니다.

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

여기서는 프로세스 컨텍스트라 잠들어도 되니 뮤텍스로 충분합니다. 인터럽트 핸들러였다면 스핀락이어야 합니다.

## 마운트 트리 순회

과제 09는 단순화한 `/proc/mounts`처럼 모든 마운트 지점을 `/proc/mymounts`에 나열합니다. procfs 파일은 읽을
때 만들어지니, 모듈은 `seq_file` 인터페이스를 씁니다. `single_open`과, 커널이 관리하는 버퍼에 출력하는
`show` 콜백입니다.

마운트를 찾는 데서 커널 버전이 중요해집니다. 옛 튜토리얼은 연결 리스트인 `mnt_ns->list`를 돕니다. 최근
커널에서는 네임스페이스의 마운트가 **레드-블랙 트리** `mnt_ns->mounts`에 있어서, 모듈은 `rb_entry`로
트리를 순회합니다.

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

솔직한 단서 두 가지. `struct mount`는 비공개 구조체라(공개 헤더가 아니라 `fs/mount.h`에서 옴) 이 모듈은
전체 커널 소스 트리에 대해서만 빌드됩니다. 그리고 순회하면서 아무 잠금도 잡지 않아서, 동시에 마운트나
언마운트가 일어나면 읽는 도중에 트리가 바뀔 수 있습니다. 진짜 `/proc/mounts`는 순회하는 동안 네임스페이스
잠금을 잡습니다.

## 남는 것

little-penguin의 대부분은 무엇이 어디에 사는지 배우는 일입니다. 로그는 `dmesg`에, 장치는 udev가 관리하는
`/dev`에, 디버그 손잡이는 debugfs에, 프로세스 정보는 procfs에. 이 지도를 알고 나면 커널 코드가 훨씬 덜
무섭습니다. 덕분에 나중에 [KFS](/ko/posts/42/kfs-rust-kernel/)에서 커널을 처음부터 만드는 일도 해 볼 만하게
느껴졌습니다.
