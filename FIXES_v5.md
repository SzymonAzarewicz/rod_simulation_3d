# Naprawy Symulacji Wędki

[Previous content from FIXES.md...]

## PIĄTA ITERACJA POPRAWEK (KRYTYCZNA - niestabilność numeryczna):

**Zgłoszony problem:**
"Na samym sracie kule nie są równo oddalone od siebie, i nagle wszystkie kule rozlatują się - wędka nie jest stabilna"

**Analiza logów - odkryto fundamentalny błąd:**

### Frame 10 (0.16s):
- Pozycje: Równo rozmieszczone ✓
- **ALE prędkości już oscylują!**
```
Point 1: vel = -0.11 (w dół)
Point 2: vel = +0.21 (w górę)
Point 3: vel = -0.27 (w dół)
Point 4: vel = +0.29 (w górę)
```
**Wzór FALI STOJĄCEJ - punkty oscylują naprzemiennie!**

### Frame 20 (0.32s): KATASTROFA
```
Point 8: vel = 20 m/s (MAX LIMIT!), acc = 7576 m/s² (750G!!!)
Point 9: vel = -20 m/s (przeciwny kierunek!)
```

### Frame 30-120: **Velocity clamping lock**
- Punkty 8 i 9 utknęły oscylując z max prędkością 20 m/s w przeciwnych kierunkach
- Przyspieszenia ~7500 m/s² (750G!)
- System próbuje eksplodować, ale velocity clamping powstrzymuje

### Frame 150 (2.4s): Wiatr włącza się (time > 2.0) → system rozpada się
```
Point 15: y = 2.41 (powinno być 4.0!)
```

### Frame 180 (2.88s): Totalna destrukcja
```
Point 15: y = -1.51 (PONIŻEJ ZIEMI!)
```

## ROOT CAUSE: NIESTABILNOŚĆ NUMERYCZNA

**Problem:** Sprężyny **ZA SZTYWNE** dla danej masy i kroku czasowego!

### Analiza częstotliwości własnej:
```
f = sqrt(k/m) / (2π)

Dla k=800 N/m, m=0.3 kg:
f = sqrt(800/0.3) / 6.28 = 8.2 Hz
T = 1/f = 0.12s (okres oscylacji)

Kryterium stabilności Eulera: dt < T/π
Wymagane: dt < 0.038s

Mieliśmy: dt = 0.016s / 2 substeps = 0.008s per substep

Teoretycznie OK, ale praktyka: NIESTABILNE!
```

**Dlaczego?**
1. Euler integration ma słabą stabilność dla sztywnych sprężyn
2. Substeps=2 za mało dla k/m ratio
3. Nawet małe zaburzenia numeryczne rosną eksponencjalnie
4. Velocity clamping maskuje problem, ale go nie rozwiązuje

## ROZWIĄZANIE: Drastyczne zmiany parametrów

### 1. ZMNIEJSZONA sztywność (6-8x mniej!):
```
Dolna 60%:    800 → 100 N/m (-87.5%)
Środkowa 20%: 400 → 50 N/m  (-87.5%)
Górna 20%:    150 → 25 N/m  (-83.3%)
```

### 2. ZWIĘKSZONE tłumienie (4x więcej!):
```
Dolna 60%:    25 → 100 Ns/m (+300%)
Środkowa 20%: 20 → 80 Ns/m  (+300%)
Górna 20%:    15 → 60 Ns/m  (+300%)
```

**Tłumienie teraz > sztywność!** To zapewnia overdamping (system kritycznie tłumiony)

### 3. ZWIĘKSZONE substeps:
```
2 → 8 substeps (+300%)
sub_dt = 0.016 / 8 = 0.002s per substep
```

### 4. WIATR WYŁĄCZONY:
- Najpierw system musi być stabilny SAM
- Wiatr można włączyć dopiero po pełnej stabilizacji
- Kod zakomentowany, gotowy do włączenia

## Nowa częstotliwość własna:

```
Dla k=100 N/m, m=0.3 kg:
f = sqrt(100/0.3) / 6.28 = 2.9 Hz
T = 0.34s

sub_dt = 0.002s << T/π = 0.11s ✓
```

**Margines bezpieczeństwa: 55x!**

## Damping ratio:

```
Critical damping: c_crit = 2*sqrt(k*m) = 2*sqrt(100*0.3) = 10.95
Actual damping: c = 100

Damping ratio: ζ = c/c_crit = 100/10.95 = 9.1

ζ > 1 = overdamped ✓
```

System jest **MOCNO NADTŁUMIONY** - wszelkie oscylacje będą szybko gasić.

## Oczekiwane zachowanie:

1. **Frame 0-10:** Minimalne oscylacje (jeśli w ogóle)
2. **Frame 10-100:** Szybkie wygaszenie oscylacji
3. **Frame 100+:** Wędka stabilna, stoi prosto
4. **Prędkości:** < 1 m/s (nie 20 m/s!)
5. **Przyspieszenia:** < 10 m/s² (nie 7500!)
6. **Pozycje:** Stałe (y = 1.0 do 4.0)

## Test stabilności:

Jeśli po tych zmianach wciąż są problemy, następne kroki:
1. Verlet integration zamiast Euler
2. Position-Based Dynamics constraints
3. Jeszcze większe tłumienie
4. Implicit Euler integration
