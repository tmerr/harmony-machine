# harmony-machine
Generates music following two rules

1. Notes played together should be simple integer ratios of one another
2. Avoid repeating notes

Can run by piping output into aplay -D pulse -r 44100 -f S16
