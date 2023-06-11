# Запуск
[Сборка для Windows](https://drive.google.com/file/d/1SuMUbWVolT9RqD-86ddP1cjN9R2mBsrR/view?usp=sharing) с CUDA.

Для запуска используется программа `launcher`.

Перед запуском необходимо выбрать настройки из папки `settings` в зависимости от конфигурации оборудования:
* При использовании видеокарты NVIDIA с 4+ ГБ памяти рекомендуются стандартные настройки;
* Для видеокарт с 2 ГБ памяти - `Settings_Low_VRAM.toml`;
* Для использования только процессора - `Settings_CPU.toml`.

Для использования файла настроек его необходимо скопировать в ту же папку, что и `launcher`, и переименовать в `Settings.toml`.

# Сборка из исходного кода (Linux)
Для выполнения требуются установленный FFmpeg, а также CUDA 11 и cuDNN 8 (если использование CUDA включено в настройках).

Команды выполняются относительно папки с проектом.

1. Скачайте [Elasticsearch](https://www.elastic.co/downloads/elasticsearch) (8.7.0);
2. Скачайте [Apache Tika](https://tika.apache.org/download.html) (`tika-server-standard-2.7.0.jar`);
3. Скачайте [ONNX Runtime](https://github.com/microsoft/onnxruntime/releases) (версии `*-gpu-*` поддерживают выполнение и на CPU, и на GPU с поддержкой CUDA);
4. Скачайте [NNServer](https://github.com/ArtSin/NNServer) и сконвертируйте модели в формат ONNX;
5. Создайте `.cargo/config.toml`, укажите в нём путь к ONNX Runtime:
```toml
[env]
ORT_RUST_STRATEGY = "system"
ORT_RUST_LIB_LOCATION = ".../onnxruntime-linux-x64-gpu-1.14.1/"
ORT_RUST_USE_CUDA = "1"
```
6. Внутри папки ONNX Runtime переместите `include/onnxruntime_c_api.h` в `include/onnxruntime/core/session/onnxruntime_c_api.h`;
7. Соберите проект из исходного кода:
   1. `cd client_ui && trunk build --release && cd ..`
   2. `cargo build --release --bin indexer`
   3. `cargo build --release --bin nn_server`
   4. `cargo build --release --bin launcher`
8. Скопируйте Elasticsearch, Apache Tika, ONNX Runtime, ONNX-модели, результаты сборки из `target/release` и файлы из `install` в какую-нибудь папку так, чтобы получилась следующая структура:
```
elasticsearch-8.7.0/
    bin/
    config/
        elasticsearch.yml
        jvm.options
        ...
    ...
nn_server/
    models/
        clip-ViT-B-32/
        clip-ViT-B-32-multilingual-v1/
        mMiniLM-L6-v2-mmarco-v2/
        paraphrase-multilingual-MiniLM-L12-v2/
    nn_server
onnxruntime-linux-x64-gpu-1.14.1/
indexer
launcher
tika-config.xml
tika-server-standard-2.7.0.jar
```