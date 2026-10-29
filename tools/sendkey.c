#include <fcntl.h>
#include <linux/input.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main(int argc, char **argv) {
    if (argc != 4) {
        fprintf(stderr, "usage: %s DEVICE CODE VALUE\n", argv[0]);
        return 2;
    }
    int fd = open(argv[1], O_WRONLY);
    if (fd < 0) { perror("open"); return 1; }
    struct input_event ev;
    memset(&ev, 0, sizeof ev);
    ev.type = EV_KEY;
    ev.code = (unsigned short)atoi(argv[2]);
    ev.value = atoi(argv[3]);
    if (write(fd, &ev, sizeof ev) != sizeof ev) { perror("write key"); return 1; }
    memset(&ev, 0, sizeof ev);
    ev.type = EV_SYN;
    if (write(fd, &ev, sizeof ev) != sizeof ev) { perror("write syn"); return 1; }
    close(fd);
    return 0;
}
