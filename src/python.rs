use crate::ai;
use crate::Direction;

use entity_gym_rs::agent::TrainEnvBuilder;
use entity_gym_rs::low_level::py_vec_env::PyVecEnv;
use pyo3::prelude::*;

#[derive(Clone)]
#[pyclass]
pub struct Config {
    pub min_action_delay: usize,
    pub max_action_delay: usize,
}

#[pymethods]
impl Config {
    #[new]
    #[args(min_action_delay = "0", max_action_delay = "0")]
    fn new(min_action_delay: usize, max_action_delay: usize) -> Self {
        Config {
            min_action_delay,
            max_action_delay,
        }
    }
}

#[pyfunction]
fn create_env(config: Config, num_envs: usize, threads: usize, first_env_index: u64) -> PyVecEnv {
    TrainEnvBuilder::default()
        .entity::<ai::Head>()
        .entity::<ai::SnakeSegment>()
        .entity::<ai::Food>()
        .action::<Direction>()
        .build_multiagent(
            config,
            super::run_headless,
            num_envs,
            threads,
            first_env_index,
        )
}

#[pymodule]
fn bevy_multisnake(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(create_env, m)?)?;
    m.add_class::<Config>()?;
    Ok(())
}
