use three_d::*;
use std::fs::OpenOptions;
use std::io::Write as IoWrite;

// Struktura reprezentująca punkt masy
#[derive(Clone, Debug)]
struct MassPoint {
    position: Vec3,
    velocity: Vec3,
    mass: f32,
    fixed: bool, // Czy punkt jest nieruchomy (np. podstawa wędki)
}

impl MassPoint {
    fn new(position: Vec3, mass: f32, fixed: bool) -> Self {
        Self {
            position,
            velocity: Vec3::zero(),
            mass,
            fixed,
        }
    }
}

// Struktura reprezentująca sprężynę
#[derive(Clone, Debug)]
struct Spring {
    point_a: usize,
    point_b: usize,
    rest_length: f32,
    stiffness: f32,
    damping: f32,
}

impl Spring {
    fn new(point_a: usize, point_b: usize, rest_length: f32, stiffness: f32, damping: f32) -> Self {
        Self {
            point_a,
            point_b,
            rest_length,
            stiffness,
            damping,
        }
    }
}

// System fizyki mass-spring
struct MassSpringSystem {
    masses: Vec<MassPoint>,
    springs: Vec<Spring>,
    gravity: Vec3,
    external_forces: Vec<Vec3>, // Siły zewnętrzne dla każdego punktu
}

impl MassSpringSystem {
    fn new(gravity: Vec3) -> Self {
        Self {
            masses: Vec::new(),
            springs: Vec::new(),
            gravity,
            external_forces: Vec::new(),
        }
    }

    fn add_mass(&mut self, mass: MassPoint) -> usize {
        self.masses.push(mass);
        self.external_forces.push(Vec3::zero());
        self.masses.len() - 1
    }

    fn add_spring(&mut self, spring: Spring) {
        self.springs.push(spring);
    }

    fn apply_force(&mut self, index: usize, force: Vec3) {
        if index < self.external_forces.len() {
            self.external_forces[index] += force;
        }
    }

    fn update(&mut self, dt: f32, log_file: &mut std::fs::File, frame: u32) {
        // Substeps dla lepszej stabilności numerycznej (ZWIĘKSZONE do 16!)
        let substeps = 16;
        let sub_dt = dt / substeps as f32;

        // Log częściej na początku (co 10 klatek), potem co 30
        let should_log = if frame <= 100 {
            frame % 10 == 0 // Pierwsze 100 klatek - co 10
        } else {
            frame % 30 == 0 // Później - co 30
        };

        if should_log {
            let _ = writeln!(log_file, "\n=== Frame {} (time={:.2}s) ===", frame, frame as f32 * 0.016);
        }

        // Przechwyć długość przed pętlą (dla borrow checkera)
        let masses_len = self.masses.len();
        let mid_point = masses_len / 2;

        for step in 0..substeps {
            // Oblicz siły sprężyn
            let mut forces: Vec<Vec3> = vec![Vec3::zero(); self.masses.len()];

            for spring in &self.springs {
                let pos_a = self.masses[spring.point_a].position;
                let pos_b = self.masses[spring.point_b].position;
                let vel_a = self.masses[spring.point_a].velocity;
                let vel_b = self.masses[spring.point_b].velocity;

                let delta = pos_b - pos_a;
                let distance = delta.magnitude();

                if distance > 0.0001 {
                    let direction = delta / distance;

                    // Siła sprężyny (prawo Hooke'a)
                    let spring_force = direction * spring.stiffness * (distance - spring.rest_length);

                    // Siła tłumienia
                    let relative_velocity = vel_b - vel_a;
                    let damping_force = direction * spring.damping * relative_velocity.dot(direction);

                    let total_force = spring_force + damping_force;

                    forces[spring.point_a] += total_force;
                    forces[spring.point_b] -= total_force;
                }
            }

            // Zastosuj siły i zaktualizuj pozycje
            for (i, mass) in self.masses.iter_mut().enumerate() {
                if !mass.fixed {
                    // Dodaj grawitację i siły zewnętrzne (tylko w pierwszym substep)
                    let mut total_force = forces[i] + self.gravity * mass.mass;
                    if step == 0 {
                        total_force += self.external_forces[i];
                    }

                    // F = ma -> a = F/m
                    let acceleration = total_force / mass.mass;

                    // Integracja Eulera z ograniczeniem prędkości
                    mass.velocity += acceleration * sub_dt;

                    // Ograniczenie maksymalnej prędkości (zapobieganie eksplozji)
                    let max_velocity = 20.0; // Zmniejszone z 50 do 20 m/s
                    let velocity_magnitude = mass.velocity.magnitude();
                    if velocity_magnitude > max_velocity {
                        mass.velocity = mass.velocity * (max_velocity / velocity_magnitude);
                    }

                    mass.position += mass.velocity * sub_dt;

                    // Loguj tylko w ostatnim substep - więcej punktów w pierwszych 10 klatkach
                    if step == substeps - 1 && should_log {
                        let should_log_point = if frame <= 10 {
                            // Pierwsze 10 klatek - loguj wszystkie punkty
                            true
                        } else {
                            // Później - tylko kluczowe punkty (0, środek, ostatni, końce sekcji)
                            i == 0 || i == masses_len - 1 || i == mid_point ||
                            i == 9 || i == 12 // Końce sekcji
                        };

                        if should_log_point {
                            let _ = writeln!(
                                log_file,
                                "Point {:2}: pos=({:6.2}, {:6.2}, {:6.2}), vel=({:6.2}, {:6.2}, {:6.2}), acc=({:6.2}, {:6.2}, {:6.2}), force=({:6.2}, {:6.2}, {:6.2})",
                                i,
                                mass.position.x, mass.position.y, mass.position.z,
                                mass.velocity.x, mass.velocity.y, mass.velocity.z,
                                acceleration.x, acceleration.y, acceleration.z,
                                total_force.x, total_force.y, total_force.z
                            );
                        }
                    }
                }
            }
        }

        // Wyczyść siły zewnętrzne po zastosowaniu
        for force in &mut self.external_forces {
            *force = Vec3::zero();
        }
    }
}

// Model wędki z kontrolą dwoma dłońmi
struct FishingRod {
    system: MassSpringSystem,
    segment_count: usize,
    bottom_grip_index: usize, // Indeks punktu trzymanego dolną dłonią (przy uchwycie)
    top_grip_index: usize,    // Indeks punktu trzymanego górną dłonią (wyżej na wędce)
}

impl FishingRod {
    fn new(base_position: Vec3, length: f32, segment_count: usize) -> Self {
        // GRAWITACJA WYŁĄCZONA (0.0 zamiast -9.81)
        let mut system = MassSpringSystem::new(vec3(0.0, 0.0, 0.0));

        let segment_length = length / segment_count as f32;
        let mass_per_segment = 0.3;

        // 3 RODZAJE SPRĘŻYN - ULTRA SŁABE + EKSTREMALNIE WYSOKIE TŁUMIENIE!
        // Strategia: Sprężyny tylko utrzymują luźny kształt, tłumienie robi całą robotę
        // Cel: Całkowicie wyeliminować oscylacje, system ultra-stabilny

        // Dolna 60% (9 segmentów z 15) - SZTYWNA podstawa (relatywnie)
        let stiff_section_end = (segment_count as f32 * 0.6).round() as usize; // 9
        let stiffness_base = 5.0;     // ULTRA SŁABE (było 50 → teraz 5, 10x słabsze!)
        let damping_base = 1000.0;    // EKSTREMALNIE WYSOKIE (było 200 → teraz 1000, 5x wyższe!)

        // Środkowa 20% (3 segmenty) - ŚREDNIA sekcja
        let medium_section_end = (segment_count as f32 * 0.8).round() as usize; // 12
        let stiffness_medium = 2.5;   // ULTRA SŁABE (było 25 → teraz 2.5, 10x słabsze!)
        let damping_medium = 800.0;   // EKSTREMALNIE WYSOKIE (było 150 → teraz 800, 5x wyższe!)

        // Górna 20% (3 segmenty) - ELASTYCZNA końcówka
        let stiffness_tip = 1.25;     // ULTRA SŁABE (było 12.5 → teraz 1.25, 10x słabsze!)
        let damping_tip = 600.0;      // EKSTREMALNIE WYSOKIE (było 100 → teraz 600, 6x wyższe!)

        // Punkty chwytów dla dwóch dłoni
        let bottom_grip_index = 0;  // Dolny uchwyt (podstawa wędki)
        let top_grip_index = 10;    // Górny chwyt (~67% wysokości, punkt 10 z 15)

        // Twórz punkty masy wzdłuż wędki
        let mut previous_index = None;
        let mut spring_index = 0;

        for i in 0..=segment_count {
            let t = i as f32 / segment_count as f32;
            let position = base_position + vec3(0.0, length * t, 0.0);
            // ŻADEN punkt nie jest "fixed" - wszystkie są kontrolowane lub swobodne
            let is_fixed = false;

            let index = system.add_mass(MassPoint::new(position, mass_per_segment, is_fixed));

            // Połącz z poprzednim punktem sprężyną
            if let Some(prev_idx) = previous_index {
                // Wybierz parametry sprężyny w zależności od sekcji
                let (stiffness, damping) = if spring_index < stiff_section_end {
                    (stiffness_base, damping_base) // Dolna 60% - sztywna
                } else if spring_index < medium_section_end {
                    (stiffness_medium, damping_medium) // Środkowa 20% - średnia
                } else {
                    (stiffness_tip, damping_tip) // Górna 20% - giętka
                };

                system.add_spring(Spring::new(
                    prev_idx,
                    index,
                    segment_length,
                    stiffness,
                    damping,
                ));

                spring_index += 1;
            }

            previous_index = Some(index);
        }

        Self {
            system,
            segment_count,
            bottom_grip_index,
            top_grip_index,
        }
    }

    // Ustaw pozycje punktów chwytów (kontrolowane przez animację dłoni)
    fn set_grip_positions(&mut self, bottom_pos: Vec3, top_pos: Vec3) {
        self.system.masses[self.bottom_grip_index].position = bottom_pos;
        self.system.masses[self.bottom_grip_index].velocity = Vec3::zero();

        self.system.masses[self.top_grip_index].position = top_pos;
        self.system.masses[self.top_grip_index].velocity = Vec3::zero();
    }

    fn update(&mut self, dt: f32, log_file: &mut std::fs::File, frame: u32) {
        // Oznacz punkty chwytów jako "fixed" podczas fizyki
        self.system.masses[self.bottom_grip_index].fixed = true;
        self.system.masses[self.top_grip_index].fixed = true;

        self.system.update(dt, log_file, frame);

        // Po aktualizacji fizyki, odznacz jako "unfixed" (dla przyszłych zmian animacji)
        self.system.masses[self.bottom_grip_index].fixed = false;
        self.system.masses[self.top_grip_index].fixed = false;
    }

    fn get_positions(&self) -> Vec<Vec3> {
        self.system.masses.iter().map(|m| m.position).collect()
    }

    // Dodaj siłę zewnętrzną do końcówki wędki (np. od ryby lub wiatru)
    fn apply_force_to_tip(&mut self, force: Vec3) {
        let last_index = self.system.masses.len() - 1;
        self.system.apply_force(last_index, force);
    }
}

fn main() {
    // Utwórz plik logowania
    let mut log_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("simulation_log.txt")
        .expect("Unable to create log file");

    writeln!(log_file, "=== Fishing Rod Simulation Log ===").unwrap();
    writeln!(log_file, "Start time: {:?}\n", std::time::SystemTime::now()).unwrap();

    // Twórz okno i kontekst
    let window = Window::new(WindowSettings {
        title: "3D Fishing Rod Simulation - Mass-Spring Model".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();

    let context = window.gl();

    // Utwórz kamerę
    let mut camera = Camera::new_perspective(
        window.viewport(),
        vec3(5.0, 3.0, 5.0),
        vec3(0.0, 2.0, 0.0),
        vec3(0.0, 1.0, 0.0),
        degrees(45.0),
        0.1,
        1000.0,
    );

    let mut control = OrbitControl::new(*camera.target(), 1.0, 100.0);

    // Twórz światło
    let light = DirectionalLight::new(&context, 2.0, Srgba::WHITE, &vec3(-1.0, -1.0, -1.0));

    // Twórz planszę (podłogę)
    let mut board_cpu_mesh = CpuMesh::square();
    board_cpu_mesh.transform(&Mat4::from_scale(10.0)).unwrap();
    board_cpu_mesh
        .transform(&Mat4::from_angle_x(degrees(-90.0)))
        .unwrap();

    let board = Gm::new(
        Mesh::new(&context, &board_cpu_mesh),
        PhysicalMaterial::new_opaque(
            &context,
            &CpuMaterial {
                albedo: Srgba::new(100, 150, 100, 255),
                ..Default::default()
            },
        ),
    );

    // Twórz siatkę na planszy
    let grid_size = 10;
    let grid_spacing = 1.0;
    let mut grid_lines = Vec::new();

    for i in 0..=grid_size {
        let offset = (i as f32 - grid_size as f32 / 2.0) * grid_spacing;
        // Linie wzdłuż osi X
        grid_lines.push(vec3(-grid_size as f32 / 2.0 * grid_spacing, 0.01, offset));
        grid_lines.push(vec3(grid_size as f32 / 2.0 * grid_spacing, 0.01, offset));
        // Linie wzdłuż osi Z
        grid_lines.push(vec3(offset, 0.01, -grid_size as f32 / 2.0 * grid_spacing));
        grid_lines.push(vec3(offset, 0.01, grid_size as f32 / 2.0 * grid_spacing));
    }

    let grid = Gm::new(
        Mesh::new(
            &context,
            &CpuMesh {
                positions: Positions::F32(grid_lines),
                ..Default::default()
            },
        ),
        ColorMaterial {
            color: Srgba::new(50, 50, 50, 255),
            ..Default::default()
        },
    );

    // Twórz wędkę - podniesiona wyżej (base na y=1.0 zamiast 0.0)
    let mut fishing_rod = FishingRod::new(vec3(0.0, 1.0, 0.0), 3.0, 15);

    // Zmienna do symulacji czasu
    let mut time = 0.0f32;
    let mut frame_count = 0u32;

    // Sterowanie animacją zamachu
    let mut casting_active = false;      // Czy zamach jest aktywny
    let mut cast_start_time = 0.0f32;    // Kiedy rozpoczął się zamach
    let cast_duration = 3.0;              // Czas trwania zamachu w sekundach (wolniej = stabilniej)

    // Pętla renderowania
    window.render_loop(move |mut frame_input| {
        let dt = 0.016; // ~60 FPS
        time += dt;
        frame_count += 1;

        // Obsługa klawiszy
        for event in frame_input.events.iter() {
            if let three_d::Event::KeyPress { kind, .. } = event {
                match *kind {
                    three_d::Key::S => {
                        // Klawisz 'S' - rozpocznij zamach
                        if !casting_active {
                            casting_active = true;
                            cast_start_time = time;
                            writeln!(log_file, "\n==========================================").unwrap();
                            writeln!(log_file, ">>> CASTING STARTED at t={:.2}s (frame {}) <<<", time, frame_count).unwrap();
                            writeln!(log_file, "==========================================\n").unwrap();
                        }
                    },
                    three_d::Key::R => {
                        // Klawisz 'R' - reset do pozycji początkowej
                        if casting_active {
                            casting_active = false;
                            writeln!(log_file, "\n>>> CASTING RESET at t={:.2}s (frame {}) <<<\n", time, frame_count).unwrap();
                        }
                    },
                    _ => {}
                }
            }
        }

        // ANIMACJA ZAMACHU WĘDKĄ (2 dłonie) - URUCHAMIANA KLAWISZEM 'S'

        // Pozycje startowe (pozycja spoczynkowa)
        let bottom_start = vec3(0.0, 1.0, 0.0);
        let top_start = vec3(0.0, 3.0, 0.0);

        // Pozycje końcowe (po zamachu) - ZMNIEJSZONE AMPLITUDY dla większej stabilności
        let bottom_end = vec3(0.3, 0.8, 0.6);    // Pcha do przodu, lekko w dół (delikatniej!)
        let top_end = vec3(-0.3, 3.5, -0.5);     // Ciągnie w tył, lekko w górę (delikatniej!)

        let (bottom_pos, top_pos) = if !casting_active {
            // STAN SPOCZYNKU - wędka stoi w pionie, czeka na klawisz 'S'
            (bottom_start, top_start)
        } else {
            // ZAMACH W TRAKCIE - animacja od początku do końca
            let anim_time = time - cast_start_time;
            let t = (anim_time / cast_duration).min(1.0); // Normalizowane 0..1

            // Smooth easing (ease-in-out cubic)
            let ease_t = if t < 1.0 {
                let t2 = t * t;
                let t3 = t2 * t;
                3.0 * t2 - 2.0 * t3
            } else {
                1.0
            };

            // Interpoluj pozycje
            let bottom = bottom_start + (bottom_end - bottom_start) * ease_t;
            let top = top_start + (top_end - top_start) * ease_t;

            (bottom, top)
        };

        // Ustaw pozycje chwytów PRZED symulacją
        fishing_rod.set_grip_positions(bottom_pos, top_pos);

        // Aktualizuj symulację wędki
        fishing_rod.update(dt, &mut log_file, frame_count);

        // Aktualizuj kontrolę kamery
        control.handle_events(&mut camera, &mut frame_input.events);

        // Twórz geometrię dla wędki
        let rod_positions = fishing_rod.get_positions();
        let mut rod_vertices = Vec::new();

        for i in 0..rod_positions.len() - 1 {
            rod_vertices.push(rod_positions[i]);
            rod_vertices.push(rod_positions[i + 1]);
        }

        let rod_mesh = Gm::new(
            Mesh::new(
                &context,
                &CpuMesh {
                    positions: Positions::F32(rod_vertices),
                    ..Default::default()
                },
            ),
            ColorMaterial {
                color: Srgba::new(139, 69, 19, 255), // Brązowy kolor
                ..Default::default()
            },
        );

        // Twórz kule w punktach masy z kolorami wg sekcji
        let mut spheres = Vec::new();
        let total_points = rod_positions.len();
        let stiff_end = (total_points as f32 * 0.6).round() as usize; // 9
        let medium_end = (total_points as f32 * 0.8).round() as usize; // 12

        for (i, pos) in rod_positions.iter().enumerate() {
            let radius = 0.05; // Wszystkie kule tego samego rozmiaru

            // Kolor w zależności od sekcji wędki i punktów chwytów
            let color = if i == 0 {
                Srgba::new(200, 50, 50, 255) // Czerwony - dolny chwyt (bottom hand)
            } else if i == 10 {
                Srgba::new(50, 100, 200, 255) // Niebieski - górny chwyt (top hand)
            } else if i <= stiff_end {
                Srgba::new(80, 40, 20, 255) // Ciemny brąz - dolna 60% (sztywna)
            } else if i <= medium_end {
                Srgba::new(139, 90, 50, 255) // Średni brąz - środkowa 20% (średnia)
            } else {
                Srgba::new(200, 180, 100, 255) // Jasny żółty-brąz - górna 20% (giętka)
            };

            let mut sphere = Gm::new(
                Mesh::new(&context, &CpuMesh::sphere(8)),
                PhysicalMaterial::new_opaque(
                    &context,
                    &CpuMaterial {
                        albedo: color,
                        ..Default::default()
                    },
                ),
            );
            sphere.set_transformation(
                Mat4::from_translation(*pos) * Mat4::from_scale(radius)
            );
            spheres.push(sphere);
        }

        // Renderuj scenę z wizualnym feedbackiem zamachu
        // Zmień kolor tła podczas zamachu (jaśniejsze niebo = casting aktywny)
        let (bg_r, bg_g, bg_b) = if casting_active {
            (0.6, 0.8, 1.0)  // Jaśniejsze niebo podczas zamachu
        } else {
            (0.5, 0.7, 1.0)  // Normalne niebo
        };

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(bg_r, bg_g, bg_b, 1.0, 1.0))
            .render(&camera, &board, &[&light])
            .render(&camera, &grid, &[&light])
            .render(&camera, &rod_mesh, &[&light])
            .render(&camera, spheres.iter().collect::<Vec<_>>().as_slice(), &[&light]);

        FrameOutput::default()
    });
}
