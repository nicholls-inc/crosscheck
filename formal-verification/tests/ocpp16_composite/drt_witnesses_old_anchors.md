# DRT report: Ocpp16Composite

Run: 2026-10-27T21:06:46Z
RNG seed: 20261024
Cases: 2000
D4 case: (a)
Recurring anchors: 2010-2016 (across leap seconds)
SUT input altered: no
SUT: EVerest lib/everest/ocpp/lib/ocpp/v16/profile.cpp at dd696165, via everest_driver
Oracle: CrosscheckModel.Ocpp16Composite via Ocpp16CompositeRunner.lean

## Verdict summary

| Status | Cases |
| --- | --- |
| Pass | 835 |
| Warn (EVerest below the model, or a limit where the model gives none) | 92 |
| Fail (EVerest above the model, or no limit, or error) | 1073 |

## Comparison counters

| Counter | Value |
| --- | --- |
| Instants compared | 78263 |
| Instants where the model gives a limit | 66541 |
| EVerest within 0.05 of the model (exact) | 56902 |
| EVerest below the model by more than 0.05 | 4331 |
| EVerest above the model, or no limit where the model has one | 5308 |
| EVerest gives a limit where the model gives none | 1081 |
| Neither gives a limit | 10641 |
| Cases where the model gives no limit at any instant | 64 |

## Flagged cases, minimised

### Case 1: fail

- Original: at 2026-10-24T03:59:58Z (offset 9898 s): model 8.9 A, EVerest 26.600000381469727 A
- Minimised: at 2026-10-24T04:59:58Z (offset 13498 s): model 26.6 A, EVerest None A
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 1`

```json
{
 "start": 1792804500,
 "length": 172800,
 "unit": "A",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 1,
   "stackLevel": 3,
   "purpose": "tx",
   "kind": "weekly",
   "startSchedule": 1399694400,
   "duration": 3600,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     266
    ]
   ]
  }
 ]
}
```

### Case 2: warn

- Original: at 2026-03-10T23:59:58Z (offset 6298 s): model no limit, EVerest 12.100000381469727 A
- Minimised: at 2026-03-10T23:59:58Z (offset 6298 s): model no limit, EVerest 12.100000381469727 A
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 2`

```json
{
 "start": 1773180900,
 "length": 172800,
 "unit": "A",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 1,
   "stackLevel": 0,
   "purpose": "tx",
   "kind": "weekly",
   "startSchedule": 1358899200,
   "duration": 172800,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     121
    ]
   ]
  }
 ]
}
```

### Case 3: fail

- Original: at 2026-08-13T10:59:59Z (offset 28799 s): model 6543.6 W, EVerest 9249.900390625 W
- Minimised: at 2026-08-13T10:59:59Z (offset 28799 s): model 6543.6 W, EVerest 9249.900390625 W
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 3`

```json
{
 "start": 1786590000,
 "length": 86400,
 "unit": "W",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 0,
   "stackLevel": 3,
   "purpose": "txDefault",
   "kind": "daily",
   "startSchedule": 1473850800,
   "duration": null,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     92499
    ],
    [
     40500,
     65436
    ]
   ]
  }
 ]
}
```

### Case 4: fail

- Original: at 2026-01-21T02:59:57Z (offset 39597 s): model 9.6 A, EVerest 27.5 A
- Minimised: at 2026-01-21T06:59:57Z (offset 53997 s): model 26.0 A, EVerest 32.0 A
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 4`

```json
{
 "start": 1768924800,
 "length": 86400,
 "unit": "A",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 1,
   "stackLevel": 4,
   "purpose": "tx",
   "kind": "daily",
   "startSchedule": 1327129200,
   "duration": null,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     320
    ],
    [
     30600,
     260
    ]
   ]
  }
 ]
}
```

### Case 5: fail

- Original: at 2026-03-21T17:29:58Z (offset 29698 s): model 10.2 A, EVerest 28.799999237060547 A
- Minimised: at 2026-03-21T11:44:58Z (offset 8998 s): model 3.5 A, EVerest 15.0 A
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 5`

```json
{
 "start": 1774084500,
 "length": 172800,
 "unit": "A",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 1,
   "stackLevel": 2,
   "purpose": "txDefault",
   "kind": "daily",
   "startSchedule": 1415187900,
   "duration": null,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     150
    ],
    [
     78300,
     35
    ]
   ]
  }
 ]
}
```

### Case 7: fail

- Original: at 2026-10-26T09:29:58Z (offset 7198 s): model 9.4 A, EVerest 17.700000762939453 A
- Minimised: at 2026-10-26T22:44:58Z (offset 54898 s): model 9.4 A, EVerest None A
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 7`

```json
{
 "start": 1792999800,
 "length": 86400,
 "unit": "A",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 0,
   "stackLevel": 2,
   "purpose": "txDefault",
   "kind": "weekly",
   "startSchedule": 1413153900,
   "duration": 86400,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     94
    ]
   ]
  }
 ]
}
```

### Case 9: fail

- Original: at 2026-06-01T17:59:58Z (offset 17998 s): model 23.1 A, EVerest 28.700000762939453 A
- Minimised: at 2026-06-01T23:29:58Z (offset 37798 s): model 26.3 A, EVerest None A
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 9`

```json
{
 "start": 1780318800,
 "length": 172800,
 "unit": "A",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 0,
   "stackLevel": 2,
   "purpose": "chargePointMax",
   "kind": "daily",
   "startSchedule": 1374838200,
   "duration": 43200,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     263
    ]
   ]
  }
 ]
}
```

### Case 11: fail

- Original: at 2026-01-20T18:29:59Z (offset 39599 s): model 2109.5 W, EVerest 9962.099609375 W
- Minimised: at 2026-01-20T21:29:59Z (offset 50399 s): model 2109.5 W, EVerest None W
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 11`

```json
{
 "start": 1768894200,
 "length": 172800,
 "unit": "W",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 1,
   "stackLevel": 3,
   "purpose": "tx",
   "kind": "daily",
   "startSchedule": 1480959000,
   "duration": 14400,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     21095
    ]
   ]
  }
 ]
}
```

### Case 12: fail

- Original: at 2026-02-11T15:44:57Z (offset 897 s): model 20.7 A, EVerest 30.200000762939453 A
- Minimised: at 2026-02-11T15:44:57Z (offset 897 s): model 15.0 A, EVerest 30.200000762939453 A
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 12`

```json
{
 "start": 1770823800,
 "length": 86400,
 "unit": "A",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 1,
   "stackLevel": 5,
   "purpose": "txDefault",
   "kind": "daily",
   "startSchedule": 1301190300,
   "duration": null,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     150
    ],
    [
     50400,
     302
    ]
   ]
  }
 ]
}
```

### Case 13: fail

- Original: at 2026-02-18T03:14:59Z (offset 18899 s): model 2195.2 W, EVerest 5806.5 W
- Minimised: at 2026-02-18T22:44:59Z (offset 89099 s): model 7462.2 W, EVerest None W
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 13`

```json
{
 "start": 1771365600,
 "length": 172800,
 "unit": "W",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 1,
   "stackLevel": 5,
   "purpose": "txDefault",
   "kind": "weekly",
   "startSchedule": 1445985900,
   "duration": 86400,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     74622
    ]
   ]
  }
 ]
}
```

### Case 14: fail

- Original: at 2026-08-23T03:44:57Z (offset 24297 s): model 10173.6 W, EVerest None W
- Minimised: at 2026-08-23T03:44:57Z (offset 24297 s): model 13865.5 W, EVerest None W
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 14`

```json
{
 "start": 1787432400,
 "length": 172800,
 "unit": "W",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 0,
   "stackLevel": 4,
   "purpose": "chargePointMax",
   "kind": "daily",
   "startSchedule": 1308354300,
   "duration": 14400,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     138655
    ]
   ]
  }
 ]
}
```

### Case 15: fail

- Original: at 2026-02-24T05:59:59Z (offset 7199 s): model 9574.2 W, EVerest 15152.7998046875 W
- Minimised: at 2026-02-24T06:14:59Z (offset 8099 s): model 15152.8 W, EVerest None W
- Replay: `python3 formal-verification/tests/ocpp16_composite/drt_harness.py --seed 20261024 --count 2000 --old-anchors --replay 15`

```json
{
 "start": 1771905600,
 "length": 86400,
 "unit": "W",
 "connector": 1,
 "profiles": [
  {
   "connectorId": 1,
   "stackLevel": 1,
   "purpose": "tx",
   "kind": "daily",
   "startSchedule": 1460440800,
   "duration": 900,
   "validFrom": null,
   "validTo": null,
   "periods": [
    [
     0,
     151528
    ]
   ]
  }
 ]
}
```

1153 more flagged cases not minimised. Indices: 16, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 34, 35, 37, 39, 40, 43, 46, 49, 51, 53, 54, 55, 56, 57, 58, 60, 62, 66, 67, 70, 72, 74, 75, 76, 77, 78, 79, 84, 85, 86, 87, 88, 91, 92, 93, 94, 96, 97, 98, 100, 101, 103, 104, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 118, 123, 124, 125, 127, 128, 129, 134, 135, 136, 137, 143, 144, 145, 146, 149, 150, 151, 154, 156, 157, 159, 160, 161, 162, 163, 164, 165, 166, 168, 170, 172, 173, 174, 175, 176, 177, 178, 180, 182, 184, 189, 190, 192, 194, 196, 197, 200, 201, 204, 205, 206, 208, 210, 211, 213, 215, 216, 217, 218, 219, 224, 225, 228, 232, 233, 235, 237, 238, 239, 240, 241, 242, 243, 244, 247, 248, 251, 254, 255, 258, 260, 262, 266, 269, 270, 272, 273, 274, 276, 277, 278, 279, 280, 285, 287, 288, 290, 293, 296, 297, 298, 300, 301, 302, 304, 305, 306, 307, 311, 312, 313, 314, 318, 320, 321, 322, 324, 325, 326, 327, 332, 333, 336, 338, 340, 342, 343, 346, 349, 350, 351, 353, 356, 357, 358, 359, 360, 362, 363, 364, 365, 367, 369, 370, 372, 373, 374, 375, 376, 377, 378, 380, 381, 384, 385, 388, 390, 392, 395, 396, 397, 398, 399, 402, 403, 404, 405, 406, 409, 410, 412, 414, 415, 417, 419, 420, 421, 423, 424, 426, 429, 430, 431, 433, 435, 436, 437, 438, 440, 442, 443, 444, 445, 446, 447, 450, 454, 455, 459, 460, 461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 473, 474, 475, 476, 478, 479, 482, 484, 485, 488, 495, 497, 498, 500, 501, 503, 504, 505, 506, 507, 509, 510, 511, 512, 514, 519, 522, 523, 524, 526, 527, 531, 532, 533, 536, 537, 538, 540, 541, 543, 544, 547, 548, 550, 555, 558, 559, 562, 563, 564, 565, 567, 568, 569, 573, 574, 575, 577, 578, 580, 583, 585, 588, 589, 592, 594, 596, 598, 601, 602, 604, 605, 606, 607, 608, 609, 610, 616, 617, 619, 621, 622, 625, 626, 627, 628, 630, 631, 632, 633, 634, 635, 636, 638, 641, 642, 643, 647, 648, 649, 650, 651, 653, 654, 655, 656, 661, 663, 664, 666, 668, 669, 673, 674, 675, 676, 677, 679, 680, 681, 682, 683, 688, 691, 693, 694, 698, 699, 700, 701, 702, 703, 704, 705, 706, 707, 708, 711, 713, 714, 717, 718, 720, 721, 723, 724, 725, 726, 729, 730, 731, 732, 733, 734, 738, 739, 740, 741, 744, 747, 749, 750, 751, 754, 755, 763, 764, 766, 768, 769, 770, 772, 773, 775, 777, 778, 779, 780, 783, 784, 785, 787, 789, 790, 791, 796, 798, 799, 800, 802, 804, 806, 810, 814, 815, 816, 819, 820, 821, 823, 825, 826, 832, 833, 835, 836, 837, 843, 844, 845, 846, 848, 849, 850, 851, 852, 853, 856, 857, 858, 859, 860, 861, 862, 863, 865, 866, 868, 871, 873, 875, 876, 877, 880, 881, 882, 883, 885, 888, 889, 890, 893, 894, 895, 896, 897, 898, 899, 901, 903, 905, 906, 907, 908, 909, 910, 912, 914, 915, 916, 917, 918, 919, 921, 923, 924, 925, 926, 927, 929, 931, 932, 933, 934, 935, 936, 937, 938, 940, 942, 947, 949, 951, 952, 953, 954, 955, 957, 959, 961, 965, 966, 967, 970, 972, 973, 974, 975, 977, 980, 981, 983, 985, 986, 987, 988, 992, 993, 997, 1000, 1001, 1002, 1003, 1005, 1006, 1009, 1011, 1013, 1016, 1017, 1018, 1019, 1020, 1021, 1023, 1027, 1028, 1029, 1030, 1031, 1032, 1033, 1034, 1036, 1037, 1038, 1039, 1040, 1042, 1045, 1049, 1051, 1053, 1056, 1059, 1061, 1062, 1063, 1064, 1065, 1066, 1067, 1070, 1072, 1074, 1078, 1079, 1080, 1081, 1086, 1088, 1092, 1093, 1097, 1099, 1100, 1105, 1108, 1113, 1114, 1115, 1118, 1120, 1121, 1122, 1124, 1127, 1130, 1131, 1133, 1134, 1138, 1139, 1143, 1144, 1146, 1147, 1149, 1150, 1153, 1154, 1158, 1160, 1163, 1165, 1167, 1168, 1169, 1170, 1172, 1173, 1176, 1177, 1178, 1181, 1182, 1183, 1184, 1186, 1187, 1190, 1191, 1192, 1193, 1196, 1198, 1199, 1200, 1201, 1203, 1205, 1206, 1208, 1213, 1214, 1215, 1217, 1219, 1220, 1222, 1224, 1225, 1226, 1228, 1229, 1230, 1232, 1235, 1236, 1237, 1238, 1242, 1243, 1244, 1245, 1246, 1247, 1249, 1254, 1255, 1256, 1257, 1260, 1261, 1262, 1263, 1264, 1265, 1272, 1273, 1278, 1279, 1280, 1282, 1283, 1286, 1288, 1289, 1290, 1292, 1297, 1299, 1300, 1302, 1304, 1307, 1310, 1311, 1312, 1314, 1315, 1319, 1322, 1323, 1324, 1325, 1326, 1327, 1333, 1339, 1341, 1343, 1347, 1348, 1351, 1355, 1357, 1358, 1362, 1366, 1367, 1368, 1369, 1372, 1373, 1374, 1375, 1377, 1378, 1379, 1380, 1382, 1383, 1385, 1386, 1388, 1390, 1391, 1394, 1395, 1396, 1397, 1398, 1400, 1402, 1404, 1405, 1409, 1411, 1412, 1413, 1414, 1417, 1418, 1419, 1421, 1425, 1426, 1428, 1430, 1431, 1432, 1438, 1439, 1440, 1441, 1444, 1445, 1448, 1449, 1450, 1452, 1453, 1455, 1458, 1463, 1464, 1465, 1466, 1469, 1470, 1473, 1474, 1476, 1481, 1483, 1484, 1485, 1486, 1489, 1490, 1491, 1492, 1493, 1494, 1497, 1498, 1500, 1502, 1503, 1504, 1507, 1508, 1509, 1511, 1514, 1515, 1516, 1517, 1519, 1520, 1523, 1524, 1525, 1526, 1528, 1529, 1530, 1531, 1532, 1534, 1536, 1538, 1540, 1542, 1543, 1544, 1545, 1546, 1547, 1548, 1549, 1550, 1551, 1552, 1553, 1555, 1556, 1560, 1561, 1562, 1564, 1565, 1567, 1568, 1570, 1571, 1572, 1573, 1574, 1575, 1576, 1578, 1579, 1580, 1582, 1584, 1585, 1587, 1588, 1589, 1591, 1593, 1595, 1596, 1598, 1599, 1602, 1603, 1604, 1610, 1611, 1612, 1614, 1616, 1617, 1619, 1620, 1623, 1626, 1627, 1628, 1629, 1630, 1631, 1632, 1633, 1634, 1636, 1638, 1640, 1642, 1643, 1644, 1645, 1647, 1652, 1654, 1655, 1657, 1658, 1660, 1662, 1663, 1664, 1665, 1667, 1669, 1670, 1672, 1673, 1677, 1678, 1679, 1680, 1681, 1682, 1684, 1685, 1686, 1687, 1688, 1689, 1694, 1696, 1699, 1700, 1701, 1703, 1705, 1706, 1707, 1708, 1709, 1710, 1712, 1714, 1716, 1717, 1718, 1721, 1726, 1727, 1729, 1732, 1733, 1736, 1739, 1740, 1742, 1745, 1746, 1747, 1748, 1749, 1751, 1753, 1754, 1755, 1756, 1757, 1759, 1762, 1765, 1766, 1767, 1769, 1770, 1771, 1776, 1777, 1780, 1782, 1784, 1785, 1787, 1788, 1789, 1793, 1794, 1796, 1798, 1799, 1800, 1801, 1804, 1805, 1808, 1810, 1811, 1813, 1814, 1817, 1819, 1821, 1824, 1825, 1826, 1827, 1829, 1830, 1831, 1832, 1834, 1839, 1841, 1843, 1845, 1847, 1849, 1850, 1852, 1853, 1855, 1859, 1860, 1861, 1867, 1869, 1870, 1871, 1872, 1875, 1876, 1878, 1881, 1883, 1887, 1888, 1890, 1891, 1892, 1893, 1894, 1895, 1902, 1903, 1904, 1906, 1907, 1908, 1909, 1910, 1911, 1912, 1913, 1914, 1915, 1916, 1919, 1922, 1923, 1924, 1927, 1928, 1929, 1931, 1932, 1933, 1936, 1938, 1939, 1940, 1941, 1943, 1946, 1947, 1949, 1951, 1957, 1959, 1962, 1964, 1965, 1967, 1971, 1973, 1974, 1975, 1976, 1978, 1981, 1982, 1983, 1986, 1987, 1988, 1990, 1991, 1993, 1994, 1995, 1996, 1998
