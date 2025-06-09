use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[macro_use]
extern crate gmod;

// Global HTTP client and async runtime
static mut CLIENT: Option<Client> = None;
static mut RUNTIME: Option<Arc<Mutex<Runtime>>> = None;

// Cache for IsRunning function
struct RunningCache {
    is_running: bool,
    last_check: Instant,
    first_check_done: bool,
}

static mut RUNNING_CACHE: Option<Arc<Mutex<RunningCache>>> = None;
const CACHE_DURATION: Duration = Duration::from_secs(2);

// Callback queue for async operations
#[derive(Debug)]
enum CallbackData {
    Generate { response: String, model: String },
    Chat { content: String, role: String, model: String },
    ListModels { models: Vec<ModelInfo> },
    GetModelInfo { license: String, modelfile: String, parameters: String, template: String },
    IsModelAvailable { is_available: bool },
    Embeddings { model: String, embeddings: Vec<Vec<f64>> },
    GetRunningModels { models: Vec<RunningModelInfo> },
    Error { message: String },
}

struct CallbackResult {
    callback_ref: i32,
    data: CallbackData,
}

static mut CALLBACK_QUEUE: Option<Arc<Mutex<Vec<CallbackResult>>>> = None;

#[derive(Serialize, Deserialize, Debug)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: Option<bool>,
    system: Option<String>,
    template: Option<String>,
    context: Option<Vec<i32>>,
    options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GenerateResponse {
    model: String,
    created_at: String,
    response: String,
    done: bool,
    context: Option<Vec<i32>>,
    total_duration: Option<u64>,
    load_duration: Option<u64>,
    prompt_eval_count: Option<u32>,
    prompt_eval_duration: Option<u64>,
    eval_count: Option<u32>,
    eval_duration: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: Option<bool>,
    options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatResponse {
    model: String,
    created_at: String,
    message: ChatMessage,
    done: bool,
    total_duration: Option<u64>,
    load_duration: Option<u64>,
    prompt_eval_count: Option<u32>,
    prompt_eval_duration: Option<u64>,
    eval_count: Option<u32>,
    eval_duration: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelInfo {
    name: String,
    modified_at: String,
    size: u64,
    digest: String,
    details: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelsResponse {
    models: Vec<ModelInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PullRequest {
    name: String,
    stream: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PullResponse {
    status: String,
    digest: Option<String>,
    total: Option<u64>,
    completed: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeleteRequest {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ShowRequest {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ShowResponse {
    license: Option<String>,
    modelfile: Option<String>,
    parameters: Option<String>,
    template: Option<String>,
    details: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmbedRequest {
    model: String,
    input: serde_json::Value, // Can be string or array of strings
    truncate: Option<bool>,
    options: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmbedResponse {
    model: String,
    embeddings: Vec<Vec<f64>>,
    total_duration: Option<u64>,
    load_duration: Option<u64>,
    prompt_eval_count: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RunningModelInfo {
    name: String,
    model: String,
    size: u64,
    digest: String,
    details: Option<serde_json::Value>,
    expires_at: Option<String>,
    size_vram: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct RunningModelsResponse {
    models: Vec<RunningModelInfo>,
}

// Configuration for Ollama connection
struct OllamaConfig {
    base_url: String,
    timeout: Duration,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            timeout: Duration::from_secs(30),
        }
    }
}

static mut CONFIG: Option<OllamaConfig> = None;

fn normalize_model_name(model_name: &str) -> String {
    if model_name.contains(':') {
        model_name.to_string()
    } else {
        format!("{}:latest", model_name)
    }
}

unsafe fn get_config() -> &'static OllamaConfig {
    CONFIG.get_or_insert_with(OllamaConfig::default)
}

unsafe fn get_client() -> &'static Client {
    CLIENT.get_or_insert_with(|| {
        Client::builder()
            .timeout(get_config().timeout)
            .build()
            .expect("Failed to create HTTP client")
    })
}

unsafe fn get_runtime() -> Arc<Mutex<Runtime>> {
    RUNTIME.get_or_insert_with(|| {
        Arc::new(Mutex::new(
            Runtime::new().expect("Failed to create async runtime")
        ))
    }).clone()
}

unsafe fn get_callback_queue() -> Arc<Mutex<Vec<CallbackResult>>> {
    CALLBACK_QUEUE.get_or_insert_with(|| {
        Arc::new(Mutex::new(Vec::new()))
    }).clone()
}

unsafe fn get_running_cache() -> Arc<Mutex<RunningCache>> {
    RUNNING_CACHE.get_or_insert_with(|| {
        Arc::new(Mutex::new(RunningCache {
            is_running: false,
            last_check: Instant::now() - CACHE_DURATION, // Force initial check
            first_check_done: false,
        }))
    }).clone()
}

unsafe fn update_running_status_async() {
    let client = get_client().clone();
    let config = get_config();
    let url = format!("{}/api/tags", config.base_url);
    let runtime = get_runtime();
    let cache = get_running_cache();

    std::thread::spawn(move || {
        let rt = runtime.lock().unwrap();
        let is_running = rt.block_on(async {
            match client.get(&url).send().await {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            }
        });

        // Update cache
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.is_running = is_running;
            cache_guard.last_check = Instant::now();
            cache_guard.first_check_done = true;
        }
    });
}

#[lua_function]
unsafe fn ollama_set_config(lua: gmod::lua::State) -> i32 {
    let base_url = lua.check_string(1).to_string();
    let timeout_secs = if lua.get_top() >= 2 && !lua.is_nil(2) {
        lua.to_number(2) as u64
    } else {
        30
    };

    CONFIG = Some(OllamaConfig {
        base_url,
        timeout: Duration::from_secs(timeout_secs),
    });

    // Reset client to use new config
    CLIENT = None;

    0
}

#[lua_function]
unsafe fn ollama_generate(lua: gmod::lua::State) -> i32 {
    let model = normalize_model_name(&lua.check_string(1));
    let prompt = lua.check_string(2).to_string();

    // Optional system prompt
    let system = if lua.get_top() >= 3 && !lua.is_nil(3) {
        Some(lua.check_string(3).to_string())
    } else {
        None
    };

        // Callback function is required
    if lua.get_top() < 4 || !lua.is_function(4) {
        lua.error("Callback function is required");
    }

    lua.push_value(4);
    let callback_ref = lua.reference();

    let request = GenerateRequest {
        model: model.clone(),
        prompt: prompt.clone(),
        stream: Some(false),
        system,
        template: None,
        context: None,
        options: None,
    };

    let client = get_client().clone();
    let config = get_config();
    let url = format!("{}/api/generate", config.base_url);
    let runtime = get_runtime();
    let queue = get_callback_queue();

    // Async execution with callback
    std::thread::spawn(move || {
        let rt = runtime.lock().unwrap();
        let result = rt.block_on(async {
            client.post(&url)
                .json(&request)
                .send()
                .await?
                .json::<GenerateResponse>()
                .await
        });

        // Queue the callback result
        let callback_result = match result {
            Ok(response) => CallbackResult {
                callback_ref,
                data: CallbackData::Generate {
                    response: response.response,
                    model: response.model,
                },
            },
            Err(e) => CallbackResult {
                callback_ref,
                data: CallbackData::Error {
                    message: format!("Error: {}", e),
                },
            },
        };

        queue.lock().unwrap().push(callback_result);
    });

    0
}

#[lua_function]
unsafe fn ollama_chat(lua: gmod::lua::State) -> i32 {
    let model = normalize_model_name(&lua.check_string(1));

    // Check if second argument is a table (messages)
    if !lua.is_table(2) {
        lua.error("Second argument must be a table of messages");
    }

        let mut messages = Vec::new();

    // Iterate through table indices until we hit nil
    let mut i = 1;
    loop {
        lua.push_integer(i as isize);
        lua.get_table(2);

        if lua.is_table(-1) {
            lua.get_field(-1, lua_string!("role"));
            lua.get_field(-2, lua_string!("content"));

            if let (Some(role), Some(content)) = (lua.get_string(-2), lua.get_string(-1)) {
                messages.push(ChatMessage {
                    role: role.to_string(),
                    content: content.to_string(),
                });
            }

            lua.pop_n(2); // Pop role and content
        }

        lua.pop(); // Pop table entry

        // If this index was nil, we've reached the end
        if lua.is_nil(-1) {
            lua.pop();
            break;
        }

        i += 1;
    }

        // Callback function is required
    if lua.get_top() < 3 || !lua.is_function(3) {
        lua.error("Callback function is required");
    }

    lua.push_value(3);
    let callback_ref = lua.reference();

    let request = ChatRequest {
        model: model.clone(),
        messages,
        stream: Some(false),
        options: None,
    };

    let client = get_client().clone();
    let config = get_config();
    let url = format!("{}/api/chat", config.base_url);
    let runtime = get_runtime();
    let queue = get_callback_queue();

    // Async execution with callback
    std::thread::spawn(move || {
        let rt = runtime.lock().unwrap();
        let result = rt.block_on(async {
            client.post(&url)
                .json(&request)
                .send()
                .await?
                .json::<ChatResponse>()
                .await
        });

        // Queue the callback result
        let callback_result = match result {
            Ok(response) => CallbackResult {
                callback_ref,
                data: CallbackData::Chat {
                    content: response.message.content,
                    role: response.message.role,
                    model: response.model,
                },
            },
            Err(e) => CallbackResult {
                callback_ref,
                data: CallbackData::Error {
                    message: format!("Error: {}", e),
                },
            },
        };

        queue.lock().unwrap().push(callback_result);
    });

    0
}

#[lua_function]
unsafe fn ollama_list_models(lua: gmod::lua::State) -> i32 {
        // Callback function is required
    if lua.get_top() < 1 || !lua.is_function(1) {
        lua.error("Callback function is required");
    }

    lua.push_value(1);
    let callback_ref = lua.reference();

    let client = get_client().clone();
    let config = get_config();
    let url = format!("{}/api/tags", config.base_url);
    let runtime = get_runtime();
    let queue = get_callback_queue();

    // Async execution with callback
    std::thread::spawn(move || {
        let rt = runtime.lock().unwrap();
        let result = rt.block_on(async {
            client.get(&url)
                .send()
                .await?
                .json::<ModelsResponse>()
                .await
        });

                // Queue the callback result
        let callback_result = match result {
            Ok(response) => CallbackResult {
                callback_ref,
                data: CallbackData::ListModels {
                    models: response.models,
                },
            },
            Err(e) => CallbackResult {
                callback_ref,
                data: CallbackData::Error {
                    message: format!("Error: {}", e),
                },
            },
        };

        queue.lock().unwrap().push(callback_result);
    });

    0
}

#[lua_function]
unsafe fn ollama_get_model_info(lua: gmod::lua::State) -> i32 {
    let model_name = normalize_model_name(&lua.check_string(1));

    // Callback function is required
    if lua.get_top() < 2 || !lua.is_function(2) {
        lua.error("Callback function is required");
    }

    lua.push_value(2);
    let callback_ref = lua.reference();

    let request = ShowRequest {
        name: model_name.clone(),
    };

    let client = get_client().clone();
    let config = get_config();
    let url = format!("{}/api/show", config.base_url);
    let runtime = get_runtime();
    let queue = get_callback_queue();

    // Async execution with callback
    std::thread::spawn(move || {
        let rt = runtime.lock().unwrap();
        let result = rt.block_on(async {
            client.post(&url)
                .json(&request)
                .send()
                .await?
                .json::<ShowResponse>()
                .await
        });

        // Queue the callback result
        let callback_result = match result {
            Ok(response) => CallbackResult {
                callback_ref,
                data: CallbackData::GetModelInfo {
                    license: response.license.unwrap_or_else(|| "".to_string()),
                    modelfile: response.modelfile.unwrap_or_else(|| "".to_string()),
                    parameters: response.parameters.unwrap_or_else(|| "".to_string()),
                    template: response.template.unwrap_or_else(|| "".to_string()),
                },
            },
            Err(e) => CallbackResult {
                callback_ref,
                data: CallbackData::Error {
                    message: format!("Error: {}", e),
                },
            },
        };

        queue.lock().unwrap().push(callback_result);
    });

    0
}

#[lua_function]
unsafe fn ollama_is_model_available(lua: gmod::lua::State) -> i32 {
    let model_name = normalize_model_name(&lua.check_string(1));

    // Callback function is required
    if lua.get_top() < 2 || !lua.is_function(2) {
        lua.error("Callback function is required");
    }

    lua.push_value(2);
    let callback_ref = lua.reference();

    let client = get_client().clone();
    let config = get_config();
    let url = format!("{}/api/tags", config.base_url);
    let runtime = get_runtime();
    let queue = get_callback_queue();

    // Async execution with callback
    std::thread::spawn(move || {
        let rt = runtime.lock().unwrap();
        let result = rt.block_on(async {
            client.get(&url)
                .send()
                .await?
                .json::<ModelsResponse>()
                .await
        });

        // Queue the callback result
        let callback_result = match result {
            Ok(response) => {
                let is_available = response.models.iter().any(|model| model.name == model_name);
                CallbackResult {
                    callback_ref,
                    data: CallbackData::IsModelAvailable { is_available },
                }
            },
            Err(e) => CallbackResult {
                callback_ref,
                data: CallbackData::Error {
                    message: format!("Error: {}", e),
                },
            },
        };

        queue.lock().unwrap().push(callback_result);
    });

    0
}

#[lua_function]
unsafe fn ollama_generate_embeddings(lua: gmod::lua::State) -> i32 {
    let model = normalize_model_name(&lua.check_string(1));

    // Second argument can be a string or table of strings
    let input = if lua.is_table(2) {
        // Handle array of strings
        let mut inputs = Vec::new();
        let mut i = 1;
        loop {
            lua.push_integer(i as isize);
            lua.get_table(2);

            if lua.is_nil(-1) {
                lua.pop();
                break;
            }

            if let Some(text) = lua.get_string(-1) {
                inputs.push(text.to_string());
            }

            lua.pop();
            i += 1;
        }
        serde_json::Value::Array(inputs.into_iter().map(serde_json::Value::String).collect())
    } else {
        // Handle single string
        let text = lua.check_string(2).to_string();
        serde_json::Value::String(text)
    };

    // Callback function is required
    if lua.get_top() < 3 || !lua.is_function(3) {
        lua.error("Callback function is required");
    }

    lua.push_value(3);
    let callback_ref = lua.reference();

    let request = EmbedRequest {
        model: model.clone(),
        input,
        truncate: Some(true),
        options: None,
    };

    let client = get_client().clone();
    let config = get_config();
    let url = format!("{}/api/embed", config.base_url);
    let runtime = get_runtime();
    let queue = get_callback_queue();

    // Async execution with callback
    std::thread::spawn(move || {
        let rt = runtime.lock().unwrap();
        let result = rt.block_on(async {
            client.post(&url)
                .json(&request)
                .send()
                .await?
                .json::<EmbedResponse>()
                .await
        });

        // Queue the callback result
        let callback_result = match result {
            Ok(response) => CallbackResult {
                callback_ref,
                data: CallbackData::Embeddings {
                    model: response.model,
                    embeddings: response.embeddings,
                },
            },
            Err(e) => CallbackResult {
                callback_ref,
                data: CallbackData::Error {
                    message: format!("Error: {}", e),
                },
            },
        };

        queue.lock().unwrap().push(callback_result);
    });

    0
}

#[lua_function]
unsafe fn ollama_get_running_models(lua: gmod::lua::State) -> i32 {
    // Callback function is required
    if lua.get_top() < 1 || !lua.is_function(1) {
        lua.error("Callback function is required");
    }

    lua.push_value(1);
    let callback_ref = lua.reference();

    let client = get_client().clone();
    let config = get_config();
    let url = format!("{}/api/ps", config.base_url);
    let runtime = get_runtime();
    let queue = get_callback_queue();

    // Async execution with callback
    std::thread::spawn(move || {
        let rt = runtime.lock().unwrap();
        let result = rt.block_on(async {
            client.get(&url)
                .send()
                .await?
                .json::<RunningModelsResponse>()
                .await
        });

        // Queue the callback result
        let callback_result = match result {
            Ok(response) => CallbackResult {
                callback_ref,
                data: CallbackData::GetRunningModels {
                    models: response.models,
                },
            },
            Err(e) => CallbackResult {
                callback_ref,
                data: CallbackData::Error {
                    message: format!("Error: {}", e),
                },
            },
        };

        queue.lock().unwrap().push(callback_result);
    });

    0
}

#[lua_function]
unsafe fn ollama_is_running(lua: gmod::lua::State) -> i32 {
    let cache = get_running_cache();

    let (is_running, needs_update, first_check) = {
        if let Ok(cache_guard) = cache.lock() {
            let needs_update = cache_guard.last_check.elapsed() >= CACHE_DURATION;
            (cache_guard.is_running, needs_update, !cache_guard.first_check_done)
        } else {
            (false, true, true) // Default to false if we can't get the lock, and trigger update
        }
    };

    // If this is the very first check, do it synchronously to get accurate result
    if first_check {
        let client = get_client().clone();
        let config = get_config();
        let url = format!("{}/api/tags", config.base_url);
        let runtime = get_runtime();

        let rt = runtime.lock().unwrap();
        let actual_status = rt.block_on(async {
            match client.get(&url).send().await {
                Ok(response) => response.status().is_success(),
                Err(_) => false,
            }
        });

        // Update cache with first check result
        if let Ok(mut cache_guard) = cache.lock() {
            cache_guard.is_running = actual_status;
            cache_guard.last_check = Instant::now();
            cache_guard.first_check_done = true;
        }

        lua.push_boolean(actual_status);
        return 1;
    }

    // If cache is stale, trigger async update
    if needs_update {
        update_running_status_async();
    }

    lua.push_boolean(is_running);
    1
}

#[lua_function]
unsafe fn process_callbacks(lua: gmod::lua::State) -> i32 {
    let queue = get_callback_queue();
    let mut callbacks = queue.lock().unwrap();

    for callback_result in callbacks.drain(..) {
        // Push error handler function that calls ErrorNoHaltWithStack
        lua.get_global(lua_string!("ErrorNoHaltWithStack"));
        let error_handler_index = lua.get_top();

        lua.from_reference(callback_result.callback_ref);

        match callback_result.data {
            CallbackData::Generate { response, model } => {
                lua.push_nil(); // No error
                lua.new_table();
                lua.push_string(&response);
                lua.set_field(-2, lua_string!("response"));
                lua.push_string(&model);
                lua.set_field(-2, lua_string!("model"));
                let _ = lua.pcall(2, 0, error_handler_index);
            },
            CallbackData::Chat { content, role, model } => {
                lua.push_nil(); // No error
                lua.new_table();
                lua.push_string(&content);
                lua.set_field(-2, lua_string!("content"));
                lua.push_string(&role);
                lua.set_field(-2, lua_string!("role"));
                lua.push_string(&model);
                lua.set_field(-2, lua_string!("model"));
                let _ = lua.pcall(2, 0, error_handler_index);
            },
            CallbackData::ListModels { models } => {
                lua.push_nil(); // No error
                lua.new_table();
                for (i, model) in models.iter().enumerate() {
                    lua.push_integer((i + 1) as isize);
                    lua.new_table();

                    lua.push_string(&model.name);
                    lua.set_field(-2, lua_string!("name"));

                    lua.push_string(&model.modified_at);
                    lua.set_field(-2, lua_string!("modified_at"));

                    lua.push_number(model.size as f64);
                    lua.set_field(-2, lua_string!("size"));

                    lua.push_string(&model.digest);
                    lua.set_field(-2, lua_string!("digest"));

                    lua.set_table(-3);
                }
                let _ = lua.pcall(2, 0, error_handler_index);
            },
            CallbackData::GetModelInfo { license, modelfile, parameters, template } => {
                lua.push_nil(); // No error
                lua.new_table();
                lua.push_string(&license);
                lua.set_field(-2, lua_string!("license"));
                lua.push_string(&modelfile);
                lua.set_field(-2, lua_string!("modelfile"));
                lua.push_string(&parameters);
                lua.set_field(-2, lua_string!("parameters"));
                lua.push_string(&template);
                lua.set_field(-2, lua_string!("template"));
                let _ = lua.pcall(2, 0, error_handler_index);
            },
            CallbackData::IsModelAvailable { is_available } => {
                lua.push_nil(); // No error
                lua.push_boolean(is_available);
                let _ = lua.pcall(2, 0, error_handler_index);
            },
            CallbackData::Embeddings { model, embeddings } => {
                lua.push_nil(); // No error
                lua.new_table();
                lua.push_string(&model);
                lua.set_field(-2, lua_string!("model"));

                // Create embeddings array
                lua.new_table();
                for (i, embedding) in embeddings.iter().enumerate() {
                    lua.push_integer((i + 1) as isize);
                    lua.new_table();
                    for (j, value) in embedding.iter().enumerate() {
                        lua.push_integer((j + 1) as isize);
                        lua.push_number(*value);
                        lua.set_table(-3);
                    }
                    lua.set_table(-3);
                }
                lua.set_field(-2, lua_string!("embeddings"));

                let _ = lua.pcall(2, 0, error_handler_index);
            },
            CallbackData::GetRunningModels { models } => {
                lua.push_nil(); // No error
                lua.new_table();
                for (i, model) in models.iter().enumerate() {
                    lua.push_integer((i + 1) as isize);
                    lua.new_table();

                    lua.push_string(&model.name);
                    lua.set_field(-2, lua_string!("name"));

                    lua.push_string(&model.model);
                    lua.set_field(-2, lua_string!("model"));

                    lua.push_number(model.size as f64);
                    lua.set_field(-2, lua_string!("size"));

                    lua.push_string(&model.digest);
                    lua.set_field(-2, lua_string!("digest"));

                    if let Some(expires_at) = &model.expires_at {
                        lua.push_string(expires_at);
                        lua.set_field(-2, lua_string!("expires_at"));
                    }

                    if let Some(size_vram) = model.size_vram {
                        lua.push_number(size_vram as f64);
                        lua.set_field(-2, lua_string!("size_vram"));
                    }

                    lua.set_table(-3);
                }
                let _ = lua.pcall(2, 0, error_handler_index);
            },
            CallbackData::Error { message } => {
                lua.push_string(&message); // Error message
                lua.push_nil();
                let _ = lua.pcall(2, 0, error_handler_index);
            },
        }

        // Clean up error handler from stack
        lua.pop();

        lua.dereference(callback_result.callback_ref);
    }

    0
}

unsafe fn initialize_callback_processor(lua: gmod::lua::State) {
    lua.get_global(lua_string!("hook"));
        lua.get_field(-1, lua_string!("Add"));
            lua.push_string("Think");
            lua.push_string("__OllamaCallbacks");
            lua.push_function(process_callbacks);
        lua.call(3, 0);
    lua.pop_n(2);
}

#[gmod13_open]
unsafe fn gmod13_open(lua: gmod::lua::State) -> i32 {
    initialize_callback_processor(lua);

    // Create Ollama table
    lua.new_table();

    // Add functions to the Ollama table
    lua.push_function(ollama_set_config);
    lua.set_field(-2, lua_string!("SetConfig"));

    lua.push_function(ollama_generate);
    lua.set_field(-2, lua_string!("Generate"));

    lua.push_function(ollama_chat);
    lua.set_field(-2, lua_string!("Chat"));

    lua.push_function(ollama_list_models);
    lua.set_field(-2, lua_string!("ListModels"));

    lua.push_function(ollama_is_running);
    lua.set_field(-2, lua_string!("IsRunning"));

    lua.push_function(ollama_get_model_info);
    lua.set_field(-2, lua_string!("GetModelInfo"));

    lua.push_function(ollama_is_model_available);
    lua.set_field(-2, lua_string!("IsModelAvailable"));

    lua.push_function(ollama_generate_embeddings);
    lua.set_field(-2, lua_string!("GenerateEmbeddings"));

    lua.push_function(ollama_get_running_models);
    lua.set_field(-2, lua_string!("GetRunningModels"));

    // Set the global Ollama table
    lua.set_global(lua_string!("Ollama"));

    0
}

#[gmod13_close]
unsafe fn gmod13_close(_: gmod::lua::State) -> i32 {
    0
}

