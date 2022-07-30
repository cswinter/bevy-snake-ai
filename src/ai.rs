use bevy::app::AppExit;
use bevy::prelude::{Assets, EventWriter, Handle, NonSendMut, Query, Res, ResMut};
use entity_gym_rs::agent::{Agent, AgentOps, Featurizable, Obs, RogueNetAgent, RogueNetAsset};

use crate::{Direction, Level, Pause, Player, Position, SnakeHead, SnakeSegments};

pub(crate) fn snake_movement_agent(
    mut players: NonSendMut<Players>,
    mut heads: Query<(&mut SnakeHead, &Position, &Player)>,
    mut exit: EventWriter<AppExit>,
    level: Query<&Level>,
    mut opponents: ResMut<Opponents>,
    opponent_handles: ResMut<OpponentHandles>,
    assets: Res<Assets<RogueNetAsset>>,
    pause: Res<Pause>,
    segments_res: Res<SnakeSegments>,
    food: Query<(&crate::Food, &Position)>,
    segment: Query<(&crate::SnakeSegment, &Position, &Player)>,
) {
    if pause.0 > 0 {
        return;
    }

    // Check if all loaded
    if opponents.0.is_empty()
        && !opponent_handles.0.is_empty()
        && opponent_handles.0.iter().all(|h| assets.get(h).is_some())
    {
        for handle in opponent_handles.0.iter() {
            let net = assets
                .get(handle)
                .unwrap()
                .agent
                .clone()
                .with_feature_adaptor::<Head>()
                .with_feature_adaptor::<SnakeSegment>()
                .with_feature_adaptor::<Food>();
            opponents.0.push(net);
        }
        println!("Loaded all opponents {:?}", opponents.0.len());
    }

    let mut head_actions = vec![];
    for (head, head_pos, player) in heads.iter_mut() {
        let obs = Obs::new(segments_res.0[player.index()].len() as f32 * 0.1)
            .entities(food.iter().map(|(_, p)| Food { x: p.x, y: p.y }))
            .entities([head_pos].iter().map(|p| Head {
                x: p.x,
                y: p.y,
                is_enemy: false,
                action_delay: head.action_delay as u64,
                action_queue: [
                    head.action_queue.get(0).into(),
                    head.action_queue.get(1).into(),
                    head.action_queue.get(2).into(),
                ],
            }))
            .entities(segments_res.0.iter().flat_map(|segments| {
                segments.iter().enumerate().map(|(i, entity)| {
                    let (_, p, plr) = segment.get(*entity).unwrap();
                    SnakeSegment {
                        x: p.x,
                        y: p.y,
                        is_enemy: player != plr,
                        distance_from_head: i as u32,
                        distance_from_tail: (segments.len() - i - 1) as u32,
                    }
                })
            }));
        match player {
            Player::Red if !opponents.0.is_empty() => {
                let level = level.iter().next().unwrap().level;
                let action = opponents.0[level - 1].act_async::<Direction>(&obs);
                head_actions.push((head, action));
            }
            _ => {
                if let Some(agent) = &mut players.0[player.index()] {
                    let action = agent.act_async::<Direction>(&obs);
                    head_actions.push((head, action));
                }
            }
        }
    }
    for (mut head, action) in head_actions.into_iter() {
        match action.rcv() {
            Some(dir) => {
                head.action_queue.push_back(dir);
                if head.action_queue.len() > head.action_delay {
                    let dir = head.action_queue.pop_front().unwrap();
                    if dir != head.direction.opposite() {
                        head.direction = dir;
                    }
                }
            }
            None => exit.send(AppExit),
        }
    }
}

pub struct Players(pub [Option<Box<dyn Agent>>; 2]);

pub struct Opponents(pub Vec<RogueNetAgent>);
pub struct OpponentHandles(pub Vec<Handle<RogueNetAsset>>);

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

#[derive(Featurizable)]
pub struct SnakeSegment {
    x: i32,
    y: i32,
    is_enemy: bool,
    distance_from_head: u32,
    distance_from_tail: u32,
}

#[derive(Featurizable)]
pub struct Food {
    x: i32,
    y: i32,
}

impl<'a> From<Option<&'a Direction>> for QueuedMove {
    fn from(dir: Option<&'a Direction>) -> Self {
        match dir {
            Some(Direction::Up) => QueuedMove::Up,
            Some(Direction::Down) => QueuedMove::Down,
            Some(Direction::Left) => QueuedMove::Left,
            Some(Direction::Right) => QueuedMove::Right,
            None => QueuedMove::None,
        }
    }
}
