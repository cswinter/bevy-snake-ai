# EntityGym AI for Bevy Snake Game

This project demonstrates how to use [EntityGym Rust](https://github.com/entity-neural-network/entity-gym-rs) to add a deep neural network opponent to [Marcus Buffett's snake clone](https://mbuffett.com/posts/bevy-snake-tutorial/).

https://user-images.githubusercontent.com/12845088/182036669-ae9e28d1-39d0-4b8e-b11b-4c8c2fc91603.mp4

## Usage

[Play the web build here](https://cswinter.github.io/bevy_snake_ai/).
Control the blue snake with WASD or the arrow keys.
Be the first snake to reach to reach a length of 10.

To run the native app, clone the repo and run:

```bash
cargo run --release
```

## Implementation

This section gives a brief overview of some AI-related implementation details.
It assumes familiarity with the [EntityGym snake tutorial][entity-gym-snake-tutorial].

### Multi-agent training

In our [previous implementation](entity-gym-snake-tutorial), we had a single agent playing the game.
Here, to allow the user to play against the AI, we train with two agents.
So we replace our `struct Player(Box<dyn Agent>)` resource with a `struct Players([Option<Box<dyn Agent>>; 2])`.
During training, both of the agents are `Some` and control the two snakes.
When running the game interactively, the first of the agents is set to `None` to allow the user to control the first snake.

To set up training with self-play between two agents, we simply change the function signature of `run_headless`:

```diff
- pub fn run_headless(_: python::Config, agents: TrainAgent, seed: u64)
+ pub fn run_headless(_: python::Config, agents: [TrainAgent; 2], seed: u64)
```

In [`src/python.rs`](src/python.rs), we can then use the [`TrainEnvBuilder::build_multiagent`](https://docs.rs/entity-gym-rs/latest/entity_gym_rs/agent/struct.TrainEnvBuilder.html#method.build_multiagent) instead of [`TrainEnvBuilder::build`](https://docs.rs/entity-gym-rs/latest/entity_gym_rs/agent/struct.TrainEnvBuilder.html#method.build) to create a Python training environment with multiple training agents.

When training with a single agent, we can use the [`AgentOps::act`](https://docs.rs/entity-gym-rs/latest/entity_gym_rs/agent/trait.AgentOps.html#method.act) method to get the action from the agent.
However, `act` will block until all agents have made their move, which means that there are multiple agents this cause us to deadlock.
Instead, we first call the nonblocking [`AgentOps::act_async`](https://docs.rs/entity-gym-rs/latest/entity_gym_rs/agent/trait.AgentOps.html#method.act_async) for every agent to get an [`ActionReceiver`](https://docs.rs/entity-gym-rs/latest/entity_gym_rs/agent/struct.ActionReceiver.html) that we can later call `recv` on to await the action.

### Action delay

If we allow the AI to instantly take an action on every frame, it plays much too quickly for humans to keep up.
To simulate human reaction speeds, we introduce a delay to all actions by the AI.
We do this by adding an `action_queue: VecDeque<Direction>` field to the `SnakeHead` entity which will queue up actions to be taken on future frames.
When the AI takes an action, instead of applying it immediately, we instead push it to the back of the queue.
Once the queue reaches the maximum length, we pop and apply the action at the front  the queue.

```rust
head.action_queue.push_back(dir);
if head.action_queue.len() >= head.action_delay {
    let dir = head.action_queue.pop_front().unwrap();
    if dir != head.direction.opposite() {
        head.direction = dir;
    }
}
```

The AI has no knowledge of anything other than the information we pass to it on each frame.
To allow the AI to take into account its past actions, we add array with all outstanding actions to the `Head` entity:

```rust
#[derive(Featurizable)]
pub struct Head {
    x: i32,
    y: i32,
    is_enemy: bool,
    action_delay: u64,
    action_queue: [QueuedMove; 3],
}

#[derive(Featurizable)]
pub enum QueuedMove {
    Up,
    Down,
    Left,
    Right,
    None,
}
```

### Multiple opponents

One of the advantages of using deep reinforcement learning is that we can easily create a series of opponents with a wide range of skill levels just by varying the amount time the AI is trained for.
The different opponents are stored in a `struct Opponents(pub Vec<RogueNetAgent>)` resource, and every time the player wins/loses a game, we increment/decrement the level counter that determines which opponent to use to control the AI.

### Asset loading

The entity-gym-rs library has an integration with Bevy's asset loading system which can be enabled by setting the "bevy" feature in [`Cargo.toml`](Cargo.toml):

```toml
[dependencies]
entity-gym-rs = { version = "0.3.1", features = ["bevy"] }
```

We register a [`RogueNetAsset`](https://docs.rs/entity-gym-rs/latest/entity_gym_rs/agent/struct.RogueNetAsset.html) and [`RogueNetAssetLoader`](https://docs.rs/entity-gym-rs/latest/entity_gym_rs/agent/struct.RogueNetAssetLoader.html) and add a `load_agents` startup system to load the model checkpoints from the [`assets/agents`](assets/agents) directory.

```rust
App::new()
    .add_asset::<RogueNetAsset>()
    .init_asset_loader::<RogueNetAssetLoader>()
    .insert_resource(OpponentHandles(vec![]))
    .add_startup_system(load_agents);

fn load_agents(mut opponent_handles: ResMut<OpponentHandles>, server: Res<AssetServer>) {
    opponent_handles.0 = [
        "500k", "1m", "2m", "4m", "8m", "16m", "32m", "64m", "128m", "256m",
    ]
    .iter()
    .map(|name| server.load(&format!("agents/{}.roguenet", name)))
    .collect();
}
```

The `AssetServer` returns handles that will eventually resolve to the actual assets, so we also have some additional code in the [`snake_movement_agent`](src/ai.rs#L7) system to check if the assets are loaded and use them as the opponents:

```rust
if opponents.0.is_empty() && opponent_handles.0.iter().all(|h| assets.get(h).is_some()) {
    for handle in opponent_handles.0.iter() {
        let net = assets.get(handle).unwrap().agent.clone();
        opponents.0.push(net);
    }
    println!("Loaded all opponents {:?}", opponents.0.len());
}
```

[entity-gym-snake-tutorial]: https://github.com/entity-neural-network/entity-gym-rs/tree/main/examples/bevy_snake
