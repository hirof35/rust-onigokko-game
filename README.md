# Rust SHADOW CHASE

macroquad を使用して作成した、視線判定と近接察知AIを搭載した2D鬼ごっこゲームです。
C++ (Siv3D) で書かれたプロトタイプをベースにRustへ移植しました。
<img width="997" height="793" alt="スクリーンショット 2026-05-17 150529" src="https://github.com/user-attachments/assets/5ca63be8-980d-4389-b680-94f554fc4a3c" />
## 実行方法

Rust環境がインストールされている状態で、以下のコマンドを実行してください。

```bash
cargo run --release
操作方法
移動: W/A/S/D または 矢印キー

ダッシュ（スタミナ消費）: Shiftキー を押しっぱなし

ルール: 鬼の視界に入るか、鬼の近く（3マス以内）に接近すると察知され、猛スピードで追いかけられます。捕まるまでの時間を競います。

