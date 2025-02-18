use bevy::{
    app::{App, Plugin, Update},
    prelude::*,
    ui::Node,
};
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use egui::Align2;
use rust_i18n::t;

use crate::{
    CleanupOnRestart, GameState, Language, Lives, OnScoreUpdate, Score, player::OnPlayerDamage,
};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (main_menu).run_if(in_state(GameState::MainMenu)))
            .add_systems(OnEnter(GameState::GameOver), handle_gameover)
            .add_systems(OnEnter(GameState::Playing), setup_hud)
            .add_observer(update_score)
            .add_observer(update_lives)
            .add_plugins((
                EguiPlugin,
                //bevy_inspector_egui::quick::WorldInspectorPlugin::default(),
            ));
    }
}
fn setup_hud(mut cmd: Commands) {
    cmd.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Center,
            padding: UiRect {
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(10.0),
                bottom: Val::Px(0.0),
            },
            ..default()
        },
        CleanupOnRestart,
    ))
    .with_child((Text::new(t!("points", count = 0)), Score::default()));
    cmd.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Start,
            padding: UiRect {
                left: Val::Px(10.0),
                right: Val::Px(0.0),
                top: Val::Px(10.0),
                bottom: Val::Px(0.0),
            },
            ..default()
        },
        CleanupOnRestart,
    ))
    .with_child((Text::new("X X X"), Lives::default()));
}

fn handle_gameover(mut cmd: Commands) {
    cmd.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            padding: UiRect {
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
            },
            flex_direction: FlexDirection::Column,
            ..default()
        },
        CleanupOnRestart,
    ))
    .with_children(|parent| {
        parent.spawn(Text::new(t!("gameover")));
        parent.spawn(Text::new(t!("goto_main_menu")));
    });
}

fn main_menu(
    mut state: ResMut<NextState<GameState>>,
    mut ctx: EguiContexts,
    mut lang: ResMut<Language>,
) {
    let rect = ctx.ctx_mut().input(|i: &egui::InputState| i.screen_rect());
    egui::Window::new("Asteroids")
        .pivot(Align2::CENTER_CENTER)
        .current_pos(egui::Pos2::new(rect.max.x / 2.0, rect.max.y / 2.0))
        .show(ctx.ctx_mut(), |ui| {
            egui::ComboBox::from_label(t!("select.language"))
                .selected_text(t!("current.language"))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut *lang, Language::English, "English");
                    ui.selectable_value(&mut *lang, Language::Polish, "Polski");
                    ui.selectable_value(&mut *lang, Language::French, "Francais");
                });
            if ui.button(t!("play")).clicked() {
                state.set(GameState::Playing);
            }
        });
    rust_i18n::set_locale(lang.locale());
}

fn update_lives(_event: Trigger<OnPlayerDamage>, mut text: Query<(&mut Text, &mut Lives)>) {
    text.iter_mut().for_each(|(mut text, mut lives)| {
        lives.0 -= 1;
        text.0 = "X ".repeat(lives.0 as usize);
    });
}

fn update_score(event: Trigger<OnScoreUpdate>, mut text: Query<(&mut Text, &mut Score)>) {
    text.iter_mut().for_each(|(mut text, mut score)| {
        score.0 += event.0;
        text.0 = t!("points", count = score.0.to_string()).to_string();
    });
}
