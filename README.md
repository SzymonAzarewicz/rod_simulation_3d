# Symulacja Wędki 3D - Model Mass-Spring

Projekt symulacji wędki w 3D oparty na modelu mass-spring (masa-sprężyna) napisany w Rust.

## Opis

Ta aplikacja przedstawia interaktywną symulację fizyki wędki wędkarskiej w środowisku 3D. Wędka jest modelowana jako system połączonych punktów masy i sprężyn, co pozwala na realistyczną symulację zachowania elastycznego pręta pod wpływem różnych sił.

## Funkcje

### 1. Plansza 3D
- Zielona płaszczyzna reprezentująca podłogę/ziemię
- Siatka pomocnicza (grid) ułatwiająca ocenę pozycji i odległości
- Rozmiar: 10x10 jednostek

### 2. Model Wędki (Mass-Spring)
Wędka składa się z:
- **15 punktów masy** połączonych sprężynami
- **Punkt bazowy** (czerwona kula) - unieruchomiony, reprezentuje podstawę wędki
- **Segmenty ruchome** (brązowe kule) - symulują elastyczność wędki

### 3. System Fizyki

#### Komponenty systemu:
- **MassPoint**: Punkt masy z pozycją, prędkością i masą
  - Pierwszy punkt jest nieruchomy (fixed = true)
  - Pozostałe punkty podlegają symulacji fizycznej

- **Spring**: Sprężyna łącząca dwa punkty
  - `rest_length`: Długość spoczynkowa sprężyny
  - `stiffness`: Sztywność (k = 500.0)
  - `damping`: Tłumienie (b = 5.0)

#### Siły działające na system:
1. **Siła sprężyny** (Prawo Hooke'a):
   ```
   F = -k * (|x| - rest_length) * direction
   ```

2. **Siła tłumienia**:
   ```
   F_damping = -b * v_relative
   ```

3. **Grawitacja**:
   ```
   F_gravity = m * g (g = -9.81 m/s²)
   ```

4. **Siła zewnętrzna** (symulacja wiatru):
   ```
   F_wind = (sin(2t) * 0.5, 0, cos(3t) * 0.3)
   ```
   - Wiatr zaczyna działać po 2 sekundach (stabilizacja)

#### Integracja numeryczna:
Metoda Eulera z krokiem czasowym dt = 0.016s (~60 FPS):
```
v(t+dt) = v(t) + a(t) * dt
x(t+dt) = x(t) + v(t) * dt
```

#### Zabezpieczenia:
- Ograniczenie maksymalnej prędkości: 50 m/s
- System sił zewnętrznych z czyszczeniem bufora
- Logowanie do pliku `simulation_log.txt`

### 4. Rendering 3D
- Biblioteka: **three-d** (v0.17)
- Kamera orbitalna z kontrolą myszą
- Oświetlenie kierunkowe
- Materiały fizyczne (PBR)

## Struktura Kodu

```
src/main.rs
├── MassPoint           - Struktura punktu masy
├── Spring              - Struktura sprężyny
├── MassSpringSystem    - System fizyki mass-spring
│   ├── new()          - Tworzy nowy system
│   ├── add_mass()     - Dodaje punkt masy
│   ├── add_spring()   - Dodaje sprężynę
│   └── update()       - Aktualizuje symulację (oblicza siły i pozycje)
├── FishingRod         - Model wędki
│   ├── new()          - Tworzy wędkę z określonymi parametrami
│   ├── update()       - Aktualizuje fizykę wędki
│   ├── get_positions() - Zwraca pozycje punktów
│   └── apply_force_to_tip() - Aplikuje siłę do końcówki
└── main()             - Główna funkcja, setup renderingu i pętla
```

## Parametry Symulacji

### Wędka:
- Długość: 3.0 jednostek
- Liczba segmentów: 15
- Masa segmentu: 0.1 kg
- Sztywność sprężyny: 500.0 N/m
- Tłumienie: 5.0 Ns/m

### Kamera:
- Pozycja startowa: (5, 3, 5)
- Target: (0, 2, 0)
- FOV: 45°
- Kontrola orbitalna (mysz)

## Budowanie i Uruchamianie

### Wymagania:
- Rust (edycja 2021 lub nowsza)
- Cargo

### Instalacja:
```bash
# Sklonuj repozytorium
git clone <repository-url>
cd rod_simulation_3d

# Zbuduj projekt
cargo build --release

# Uruchom symulację
cargo run --release
```

### Zależności:
```toml
[dependencies]
three-d = "0.17"
```

## Sterowanie

- **Mysz (lewy przycisk + przeciągnięcie)**: Obracanie kamery wokół sceny
- **Scroll myszy**: Przybliżanie/oddalanie widoku
- **ESC**: Zamknięcie aplikacji

## Debugowanie

Aplikacja automatycznie tworzy plik `simulation_log.txt` z informacjami diagnostycznymi:

```bash
# Podgląd logów w czasie rzeczywistym
tail -f simulation_log.txt

# Analiza logów po uruchomieniu
cat simulation_log.txt
```

**Format logów:**
- Logowanie co 0.5 sekundy (30 klatek)
- Punkty: podstawa (0), środek (7-8), końcówka (15)
- Dane: pozycja, prędkość, przyspieszenie, siła całkowita

**Sprawdzanie problemów:**
- Prędkości > 50 m/s → ograniczenie aktywne
- Pozycje z NaN/Inf → błąd numeryczny
- Wszystkie punkty na (0,0,0) → problem inicjalizacji

## Możliwe Rozszerzenia

1. **Interaktywność**:
   - Możliwość ręcznego aplikowania sił myszą
   - Dodanie interfejsu do zmiany parametrów symulacji w czasie rzeczywistym

2. **Rozszerzona fizyka**:
   - Kolizje z obiektami
   - Symulacja żyłki wędkarskiej
   - Model ryby na końcu żyłki

3. **Wizualizacja**:
   - Lepsza tekstura wędki (drewno)
   - Animacja rzucania
   - Cząsteczki wody

4. **Dokładność fizyczna**:
   - Integracja Verlet zamiast Eulera (większa stabilność)
   - Ograniczenia kątowe między segmentami
   - Symulacja zginania i skręcania

## Teoria: Model Mass-Spring

Model mass-spring to powszechnie stosowana metoda w grafice komputerowej i symulacjach fizycznych do reprezentowania obiektów odkształcalnych.

### Zalety:
- Prosty w implementacji
- Intuicyjny (bezpośrednie odwzorowanie prawa Hooke'a)
- Elastyczny - łatwo dodawać nowe punkty i sprężyny

### Wady:
- Może być niestabilny przy dużych siłach
- Wymaga małego kroku czasowego dla dokładności
- Super-elastyczne zachowanie przy wysokich prędkościach

### Zastosowania:
- Symulacja tkanin
- Modele włosów i futra
- Animacja postaci (soft body)
- Symulacja lin, kabli i... wędek!

## Autor

Projekt stworzony jako demonstracja symulacji fizycznej w Rust z użyciem modelu mass-spring.

## Licencja

MIT
