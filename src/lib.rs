use netsblox_vm::real_time::*;
use netsblox_vm::std_system::*;
use netsblox_vm::std_util::*;
use netsblox_vm::bytecode::*;
use netsblox_vm::process::*;
use netsblox_vm::runtime::*;
use netsblox_vm::project::*;
use netsblox_vm::gc::*;
use netsblox_vm::ast;

use std::time::Duration;
use std::sync::Arc;
use std::rc::Rc;

mod note;

// -----------------------------------------------------------------

const BASE_URL: &'static str = "https://cloud.netsblox.org";

const CLOCK_INTERVAL: Duration = Duration::from_millis(10);
const RUN_TIME_INTERVAL: Duration = Duration::from_secs(2);

const YIELDS_BEFORE_SLEEP: usize = 64;
const IDLE_SLEEP_TIME: Duration = Duration::from_millis(1);

// -----------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeType {} // type enum for a NativeValue - we don't have any native values we want to expose, so just use an empty enum

#[derive(Debug)]
enum NativeValue {} // enum for native values that are exposed to the vm - we don't have any we want to expose, so just use an empty enum
impl GetType for NativeValue {
    type Output = NativeType;
    fn get_type(&self) -> Self::Output {
        unreachable!() // because we don't have any native values to get the type of
    }
}

struct EntityState; // a type to hold custom entity (sprite or stage) state - we don't have any, so just use a unit struct
impl From<EntityKind<'_, '_, C, StdSystem<C>>> for EntityState {
    fn from(_: EntityKind<'_, '_, C, StdSystem<C>>) -> Self {
        EntityState
    }
}

struct ProcessState; // a type to hold custom process (script) state - we don't have any, so just use a unit struct
impl From<ProcessKind<'_, '_, C, StdSystem<C>>> for ProcessState {
    fn from(_: ProcessKind<'_, '_, C, StdSystem<C>>) -> Self {
        ProcessState
    }
}

struct C; // a type to hold all of our custom type definitions for the vm to use
impl CustomTypes<StdSystem<C>> for C {
    type NativeValue = NativeValue; // a type to hold any native rust values exposed to the vm
    type Intermediate = SimpleValue; // a Send type that serves as an intermediate between vm gc values and normal rust

    type EntityState = EntityState; // a type to hold the custom state for an entity (sprite or stage)
    type ProcessState = ProcessState; // a type to hold the custom state for a process (script)

    // a function to convert intermediate values into native vm values
    fn from_intermediate<'gc>(mc: &Mutation<'gc>, value: Self::Intermediate) -> Value<'gc, C, StdSystem<C>> {
        Value::from_simple(mc, value)
    }
}

// our top-level gc arena - this will hold our gc-allocated project and everything it contains
#[derive(Collect)]
#[collect(no_drop)]
struct Env<'gc> {
                               proj: Gc<'gc, RefLock<Project<'gc, C, StdSystem<C>>>>,
    #[collect(require_static)] locs: Locations, // bytecode locations info for generating error traces
}
type EnvArena = Arena<Rootable![Env<'_>]>;

// converts a netsblox xml project containing a single role into a new gc environment object containing a running project
fn get_running_project(xml: &str, system: Rc<StdSystem<C>>) -> EnvArena {
    EnvArena::new(|mc| {
        let parser = ast::Parser::default();
        let ast = parser.parse(xml).unwrap();
        assert_eq!(ast.roles.len(), 1); // this should be handled more elegantly in practice - for the sake of this example, we only allow one role

        let (bytecode, init_info, locs, _) = ByteCode::compile(&ast.roles[0]).unwrap();

        let mut proj = Project::from_init(mc, &init_info, Rc::new(bytecode), Settings::default(), system);
        proj.input(mc, Input::Start); // this is equivalent to clicking the green flag button

        Env { proj: Gc::new(mc, RefLock::new(proj)), locs }
    })
}

pub fn netsblox_to_musicxml(xml: &str) -> String {
    // create a new shared clock and start a thread that updates it at our desired interval
    let clock = Arc::new(Clock::new(UtcOffset::UTC, Some(Precision::Medium)));
    let clock_clone = clock.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(CLOCK_INTERVAL);
        clock_clone.update();
    });

    // create a custom config for the system
    let config = Config::<C, StdSystem<C>> {
        request: Some(Rc::new(move |_mc, key, request, _proc| match &request {
            Request::UnknownBlock { name, args } => {
                match name.as_str() {
                    "playNote" => {
                        assert_eq!(args.len(), 3);
                        let pitch = args[2].to_string();
                        let duration = format!("{}{}", args[1], args[0]);
                        let note = note::note_builder(pitch, duration);
                        println!("{}", note::to_string(&note));
                        key.complete(Ok(SimpleValue::String("OK".into()).into()));
                    }
                    _ => return RequestStatus::UseDefault { key, request },
                }
                RequestStatus::Handled
            }
            _ => RequestStatus::UseDefault { key, request },
        })),
        command: None,
    };

    // initialize our system with all the info we've put together
    let system = Rc::new(StdSystem::new_sync(BASE_URL.to_owned(), None, config, clock.clone()));
    let env = get_running_project(&xml, system);

    // begin running the code - these are some helpers to make things more efficient in terms of memory and cpu resources
    let mut idle_sleeper = IdleAction::new(YIELDS_BEFORE_SLEEP, Box::new(|| std::thread::sleep(IDLE_SLEEP_TIME)));
    let stop_time = clock.read(Precision::Medium) + RUN_TIME_INTERVAL;
    loop {
        env.mutate(|mc, env| {
            let mut proj = env.proj.borrow_mut(mc);
            for _ in 0..1024 {
                // step the virtual machine forward by one bytecode instruction
                let res = proj.step(mc);
                if let ProjectStep::Error { error, proc } = &res {
                    // if we get an error, we can generate an error summary including a stack trace - here we just print out the result
                    let trace = ErrorSummary::extract(error, proc, &env.locs);
                    println!("error: {error:?}\ntrace: {trace:?}");
                }
                // this takes care of performing thread sleep if we get a bunch of no-ops from proj.step back to back
                idle_sleeper.consume(&res);
            }
        });
        // if it's time for us to do garbage collection, do it and reset the next collection time
        if clock.read(Precision::Low) >= stop_time {
            break;
        }
    }
    unimplemented!();
}