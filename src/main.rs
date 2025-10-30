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

        // Log co 30 klatek (0.5 sekundy przy 60 FPS)
        let should_log = frame % 30 == 0;

        if should_log {
            let _ = writeln!(log_file, "\n=== Frame {} ===", frame);
        }

        // Zastosuj siły i zaktualizuj pozycje
        for (i, mass) in self.masses.iter_mut().enumerate() {
            if !mass.fixed {
                // Dodaj grawitację i siły zewnętrzne
                let total_force = forces[i] + self.gravity * mass.mass + self.external_forces[i];

                // F = ma -> a = F/m
                let acceleration = total_force / mass.mass;

                // Integracja Eulera z ograniczeniem prędkości
                mass.velocity += acceleration * dt;

                // Ograniczenie maksymalnej prędkości (zapobieganie eksplozji)
                let max_velocity = 50.0;
                let velocity_magnitude = mass.velocity.magnitude();
                if velocity_magnitude > max_velocity {
                    mass.velocity = mass.velocity * (max_velocity / velocity_magnitude);
                }

                mass.position += mass.velocity * dt;

                if should_log && (i == 0 || i == self.masses.len() - 1 || i == self.masses.len() / 2) {
                    let _ = writeln!(
                        log_file,
                        "Point {}: pos=({:.2}, {:.2}, {:.2}), vel=({:.2}, {:.2}, {:.2}), acc=({:.2}, {:.2}, {:.2}), force=({:.2}, {:.2}, {:.2})",
                        i,
                        mass.position.x, mass.position.y, mass.position.z,
                        mass.velocity.x, mass.velocity.y, mass.velocity.z,
                        acceleration.x, acceleration.y, acceleration.z,
                        total_force.x, total_force.y, total_force.z
                    );
                }
            }
        }

        // Wyczyść siły zewnętrzne po zastosowaniu
        for force in &mut self.external_forces {
            *force = Vec3::zero();
        }
    }
}

// Model wędki
struct FishingRod {
    system: MassSpringSystem,
    segment_count: usize,
}

impl FishingRod {
    fn new(base_position: Vec3, length: f32, segment_count: usize) -> Self {
        let mut system = MassSpringSystem::new(vec3(0.0, -9.81, 0.0));

        let segment_length = length / segment_count as f32;
        let mass_per_segment = 0.1;
        let stiffness = 500.0;
        let damping = 5.0;

        // Twórz punkty masy wzdłuż wędki
        let mut previous_index = None;
        for i in 0..=segment_count {
            let t = i as f32 / segment_count as f32;
            let position = base_position + vec3(0.0, length * t, 0.0);
            let is_fixed = i == 0; // Pierwszy punkt jest nieruchomy (podstawa)

            let index = system.add_mass(MassPoint::new(position, mass_per_segment, is_fixed));

            // Połącz z poprzednim punktem sprężyną
            if let Some(prev_idx) = previous_index {
                system.add_spring(Spring::new(
                    prev_idx,
                    index,
                    segment_length,
                    stiffness,
                    damping,
                ));
            }

            previous_index = Some(index);
        }

        Self {
            system,
            segment_count,
        }
    }

    fn update(&mut self, dt: f32, log_file: &mut std::fs::File, frame: u32) {
        self.system.update(dt, log_file, frame);
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

    // Twórz wędkę
    let mut fishing_rod = FishingRod::new(vec3(0.0, 0.0, 0.0), 3.0, 15);

    // Zmienna do symulacji czasu
    let mut time = 0.0f32;
    let mut frame_count = 0u32;

    // Pętla renderowania
    window.render_loop(move |mut frame_input| {
        let dt = 0.016; // ~60 FPS
        time += dt;
        frame_count += 1;

        // Zastosuj siłę do końcówki wędki (symulacja wiatru - znacznie zmniejszona)
        // Siła jest teraz opcjonalna i dużo mniejsza
        if time > 2.0 { // Zacznij od 2 sekundy, żeby wędka się ustabilizowała
            let wind_force = vec3(
                (time * 2.0).sin() * 0.5,  // Zmniejszone z 2.0 do 0.5
                0.0,
                (time * 3.0).cos() * 0.3,  // Zmniejszone z 1.5 do 0.3
            );
            fishing_rod.apply_force_to_tip(wind_force);
        }

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

        // Twórz kule w punktach masy
        let mut spheres = Vec::new();
        for (i, pos) in rod_positions.iter().enumerate() {
            let radius = if i == 0 { 0.08 } else { 0.05 }; // Większa kula u podstawy
            let mut sphere = Gm::new(
                Mesh::new(&context, &CpuMesh::sphere(8)),
                PhysicalMaterial::new_opaque(
                    &context,
                    &CpuMaterial {
                        albedo: if i == 0 {
                            Srgba::new(200, 50, 50, 255)
                        } else {
                            Srgba::new(139, 69, 19, 255)
                        },
                        ..Default::default()
                    },
                ),
            );
            sphere.set_transformation(
                Mat4::from_translation(*pos) * Mat4::from_scale(radius)
            );
            spheres.push(sphere);
        }

        // Renderuj scenę
        frame_input
            .screen()
            .clear(ClearState::color_and_depth(0.5, 0.7, 1.0, 1.0, 1.0))
            .render(&camera, &board, &[&light])
            .render(&camera, &grid, &[&light])
            .render(&camera, &rod_mesh, &[&light])
            .render(&camera, spheres.iter().collect::<Vec<_>>().as_slice(), &[&light]);

        FrameOutput::default()
    });
}
