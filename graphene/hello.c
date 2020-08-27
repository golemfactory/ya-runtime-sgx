#include <err.h>
#include <fcntl.h>
#include <stdio.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <unistd.h>

int main(int argc, char* argv[]) {
    puts("HELLO WORLD");

    if (argc != 3) {
        errx(1, "invalid argc: %d", argc);
    }

    int in_fd = open(argv[1], O_RDONLY);
    if (in_fd < 0) {
        err(1, "open in");
    }
    int out_fd = open(argv[2], O_WRONLY | O_CREAT | O_EXCL, 0777);
    if (out_fd < 0) {
        err(1, "open out");
    }

    while (1) {
        char buf[0x1000];
        ssize_t x = read(in_fd, buf, sizeof buf);
        if (x < 0) {
            err(1, "read");
        } else if (x == 0) {
            break;
        }
        if (write(out_fd, buf, x) != x) {
            err(1, "write");
        }
    }

    puts("COPY DONE");

    return 0;
}
