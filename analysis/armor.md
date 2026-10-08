# Armor

Written by `lab armor 60` (SIM_VERSION 18). The yardstick against each opponent on the road, the same 60 seeds with the opponent dressed (data/costumes.json as it stands) and undressed (no plate, no costume). A plate hit is a blade first meeting a plate. Step time is the simulation alone plus both pilots, per tick, on this machine, threads running side by side.

| opponent | plates | yardstick wins (dressed / bare) | mean ticks a match | rounds lost to the head or neck | plate hits a round | cuts it took a round | µs a tick (dressed / bare) | same matches |
|---|---|---|---|---|---|---|---|---|
| bridge_keeper | sombrero | 10 (17 %) / 15 (25 %) | 883 / 854 | 38 % / 42 % | 0.44 | 3.84 / 3.99 | 9441.7 / 8310.3 | 10 of 60 |
| gatekeeper | skullcap | 26 (43 %) / 29 (48 %) | 962 / 843 | 55 % / 60 % | 0.48 | 1.75 / 1.85 | 4198.4 / 3673.8 | 1 of 60 |
| general | helmet, breastplate | 0 (0 %) / 0 (0 %) | 408 / 383 | 50 % / 50 % | 0.50 | 0.49 / 0.36 | 6409.3 / 5890.3 | 4 of 60 |
| ox_herd | horns | 51 (85 %) / 44 (73 %) | 414 / 402 | 53 % / 65 % | 0.38 | 2.81 / 2.33 | 4722.3 / 4514.2 | 0 of 60 |
| temple_guard | helmet | 23 (38 %) / 33 (55 %) | 607 / 570 | 57 % / 72 % | 0.58 | 1.85 / 2.24 | 4549.9 / 4201.6 | 0 of 60 |
| warden | helmet | 14 (23 %) / 16 (27 %) | 643 / 660 | 24 % / 63 % | 0.45 | 1.40 / 1.55 | 5890.7 / 5237.6 | 0 of 60 |
| watchman | helmet | 15 (25 %) / 7 (12 %) | 654 / 658 | 34 % / 45 % | 0.38 | 1.32 / 1.06 | 5043.6 / 4610.4 | 3 of 60 |
| herbalist | none | 22 (37 %) / 22 (37 %) | 568 / 568 | 52 % / 52 % | 0.00 | 1.52 / 1.52 | 5957.5 / 5403.9 | 60 of 60 |
| abbot | none | 13 (22 %) / 13 (22 %) | 657 / 657 | 44 % / 44 % | 0.00 | 1.24 / 1.24 | 5439.7 / 4902.1 | 60 of 60 |
| scarecrow | none | 60 (100 %) / 60 (100 %) | 452 / 452 | 84 % / 84 % | 0.00 | 3.84 / 3.84 | 3494.4 / 3482.9 | 60 of 60 |
| pilgrim | none | 57 (95 %) / 57 (95 %) | 608 / 608 | 57 % / 57 % | 0.00 | 2.92 / 2.92 | 4217.5 / 4020.4 | 60 of 60 |
