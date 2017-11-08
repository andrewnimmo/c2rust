// Should fail

void entry(unsigned buffer_size, int buffer[])
{
    if (buffer_size >= 10) {
        int i = 0;

        // Test break
        while (1) {
            if (i > 7) break;
            buffer[i++] = 1;
        }

        // Test continue
        do {
            buffer[i++] = 2;
            if (i < 10) continue;
        } while (0);

        // Test continue, ensure it runs the increment
        for (i = 20; i < 30; i++) {
                i++;
                if (i < 25) continue;
                buffer[i] = 3;
        }
    }
}
