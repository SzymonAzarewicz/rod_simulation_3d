# Naprawy Symulacji Wędki

## Zidentyfikowane Problemy

### 1. **Błąd aplikacji siły do końcówki** (KRYTYCZNY)
**Lokalizacja:** `apply_force_to_tip()` - linia 173 (stara wersja)

**Problem:**
```rust
last.velocity += force / last.mass * 0.016;
```

Ta implementacja była fundamentalnie błędna:
- `force / mass` daje przyspieszenie (a = F/m)
- Mnożenie przez `0.016` (dt) aplikowało zmianę prędkości
- Następnie w `update()` integracja Eulera ponownie mnożyła przez dt
- **Efekt:** Siła była aplikowana z dt², powodując eksplozję numeryczną

**Rozwiązanie:**
- Dodano wektor `external_forces` w `MassSpringSystem`
- Metoda `apply_force()` dodaje siłę do bufora
- Siły są aplikowane prawidłowo w `update()` wraz z innymi siłami
- Buffer jest czyszczony po każdej klatce

### 2. **Zbyt duże siły wiatru**
**Lokalizacja:** główna pętla renderowania

**Problem:**
```rust
let wind_force = vec3(
    (time * 2.0).sin() * 2.0,  // 2.0 N
    0.0,
    (time * 3.0).cos() * 1.5,  // 1.5 N
);
```

Przy masie segmentu 0.1 kg, siła 2.0 N daje przyspieszenie 20 m/s² - zbyt duże!

**Rozwiązanie:**
- Zmniejszono siły wiatru do 0.5 N i 0.3 N (4x mniejsze)
- Dodano opóźnienie 2 sekundy przed aplikacją wiatru (stabilizacja)

### 3. **Brak zabezpieczeń przed eksplozją numeryczną**

**Problem:**
- Brak ograniczenia prędkości
- Integracja Eulera może być niestabilna przy dużych siłach

**Rozwiązanie:**
- Dodano ograniczenie maksymalnej prędkości (50 m/s)
- Velocity clamping zapobiega eksplozji układu

## Dodane Funkcje

### 1. **Logowanie do pliku**
Plik: `simulation_log.txt`

**Format:**
```
=== Frame 30 ===
Point 0: pos=(x, y, z), vel=(x, y, z), acc=(x, y, z), force=(x, y, z)
Point 7: pos=(x, y, z), vel=(x, y, z), acc=(x, y, z), force=(x, y, z)
Point 15: pos=(x, y, z), vel=(x, y, z), acc=(x, y, z), force=(x, y, z)
```

**Szczegóły:**
- Logowanie co 30 klatek (~0.5 sekundy)
- Loguje punkty: pierwszy (0), środkowy (7/8), ostatni (15)
- Zawiera pozycję, prędkość, przyspieszenie i siłę całkowitą

### 2. **System sił zewnętrznych**
- `external_forces: Vec<Vec3>` w `MassSpringSystem`
- `apply_force(index, force)` - dodaje siłę do punktu
- Automatyczne czyszczenie po każdej klatce

## Parametry Fizyczne

### DRUGA ITERACJA POPRAWEK (po analizie logów):

**Problem zidentyfikowany z logów:**
- Frame 30: przyspieszenia ~7000 m/s² (700G!)
- Frame 180+: przyspieszenia do 12000 m/s² (1200G!)
- Prędkości natychmiast osiągały limit 50 m/s
- Siły rzędu 700-1200 N dla masy 0.1 kg = katastrofa

**Przyczyna:** Sprężyny zbyt sztywne + masa zbyt mała = niestabilność numeryczna

### Nowe wartości (v2):
- **Masa segmentu:** 0.3 kg (↑ z 0.1, 3x cięższe)
- **Sztywność sprężyny:** 100 N/m (↓ z 500, 5x mniejsza)
- **Tłumienie:** 15.0 Ns/m (↑ z 5, 3x większe)
- **Grawitacja:** -9.81 m/s²
- **Max prędkość:** 20 m/s (↓ z 50)
- **Siła wiatru:** 0.3-0.5 N (po stabilizacji)
- **Substeps:** 4 (nowe! dt/4 dla każdego kroku)

### Uzasadnienie zmian:

**Masa 0.1 → 0.3 kg:**
- Większa masa = mniejsze przyspieszenia przy tej samej sile
- a = F/m, więc 3x większa masa = 3x mniejsze przyspieszenie
- Bardziej stabilne numerycznie

**Stiffness 500 → 100 N/m:**
- Przy rozciągnięciu 0.5m: F = 100×0.5 = 50 N (zamiast 250 N)
- Przyspieszenie: a = 50/0.3 = 167 m/s² (zamiast 2500 m/s²!)
- 15x mniejsze przyspieszenie!

**Damping 5 → 15 Ns/m:**
- Większe tłumienie pochłania więcej energii
- Szybsza stabilizacja oscylacji
- Zapobiega drżeniom

**Substeps 1 → 4:**
- Zamiast jednego kroku 0.016s, robimy 4 kroki po 0.004s
- Mniejszy krok czasowy = lepsza precyzja numeryczna
- Sprężyny są przeliczane 4x częściej = bardziej dokładne siły

**Max velocity 50 → 20 m/s:**
- Bardziej realistyczny limit
- Jeszcze większe bezpieczeństwo przed eksplozją

### Oczekiwane zachowanie:
1. Pierwsze 2 sekundy: wędka opada pod wpływem grawitacji
2. System stabilizuje się w pozycji wiszącego pręta
3. Po 2 sekundach: delikatne kołysanie od wiatru

## Testy do wykonania

1. **Uruchom symulację:**
   ```bash
   cargo run --release
   ```

2. **Sprawdź log:**
   ```bash
   tail -f simulation_log.txt
   ```

3. **Obserwuj:**
   - Wszystkie 16 kul powinno być widocznych
   - Wędka powinna zwisać w dół
   - Delikatne kołysanie po 2 sekundach
   - Brak eksplozji lub znikania punktów

## Możliwe dalsze ulepszenia

1. **Stabilniejsza integracja:**
   - Verlet integration zamiast Eulera
   - Adaptive timestep

2. **Lepsza wizualizacja:**
   - Cylindry zamiast linii dla wędki
   - Tekstury
   - Trail effect dla końcówki

3. **Interaktywność:**
   - Kliknij i przeciągnij punkty myszą
   - Regulatory parametrów w czasie rzeczywistym
   - Pause/play
