sed -i 's/fs::remove_file("test_corpus.h5").ok();/std::fs::remove_file("test_corpus.h5").ok();/g' src/ml/event_corpus.rs
sed -i 's/let model = Wav2Vec2Model::<Wgpu>::load(&device).await.unwrap();/let model = Wav2Vec2Model::<burn::backend::Wgpu>::load(\&device).await.unwrap();/g' src/ml/wav2vec2_loader.rs
