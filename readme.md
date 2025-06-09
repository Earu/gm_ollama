# gm_ollama - Ollama Integration for Garry's Mod

A Garry's Mod binary module that provides Lua bindings for the [Ollama](https://ollama.ai/) API, allowing you to connect to local LLMs directly in your GMod server or client.

## Prerequisites

- [Ollama](https://ollama.ai/) installed and running
- Rust toolchain for building
- Garry's Mod

## Building

1. Clone this repository
2. Make sure you have Rust installed with the correct toolchain:
   ```bash
   rustup install stable
   rustup default stable
   ```
3. Build the module:
   ```bash
   cargo build --release
   ```
4. The compiled binary will be in `target/release/`

## Installation

1. Place the compiled binary in your GMod's `garrysmod/lua/bin/` folder
2. Rename it to follow GMod's naming convention:
   - Windows (64-bit): `gmsv_ollama_win64.dll` (server) or `gmcl_ollama_win64.dll` (client)
   - Linux (64-bit): `gmsv_ollama_linux64.dll` (server) or `gmcl_ollama_linux64.dll` (client)
   - Windows (32-bit): `gmsv_ollama_win32.dll` (server) or `gmcl_ollama_win32.dll` (client)

## API Reference

**Important**: All operations are asynchronous. Callbacks use the error-first pattern: `function(err, data)`.

### Configuration

#### `Ollama.SetConfig(url, timeout)`
Configure the Ollama connection.
- `url` (string): Ollama server URL (default: "http://localhost:11434")
- `timeout` (number): Request timeout in seconds (default: 30)

```lua
Ollama.SetConfig("http://localhost:11434", 30)
```

#### `Ollama.IsRunning()`
Check if Ollama server is accessible. Returns cached result (updated every 2 seconds).
- Returns: `boolean` - true if accessible

```lua
if Ollama.IsRunning() then
    print("Ollama is running!")
else
    print("Ollama is not running or not accessible")
end
```

### Text Generation

#### `Ollama.Generate(model, prompt, system, callback)`
Generate text using the specified model. Model names without tags automatically get ":latest" appended.

**Parameters:**
- `model` (string): Model name (e.g., "llama2", "codellama:13b")
- `prompt` (string): The text prompt
- `system` (string, optional): System prompt to guide behavior (can be nil)
- `callback` (function): Callback function `function(err, data)`

**Callback data structure:**
```lua
{
    response = "Generated text response",
    model = "llama2:latest"
}
```

**Example:**
```lua
Ollama.Generate("llama2", "Tell me a short joke about programming", nil, function(err, data)
    if err then
        print("Error: " .. err)
    else
        print("Response from " .. data.model .. ": " .. data.response)
    end
end)

-- With system prompt
Ollama.Generate("llama2", "Explain quantum physics", "You are a helpful physics teacher. Keep explanations simple.", function(err, data)
    if err then
        print("Error: " .. err)
    else
        print("Physics Response: " .. data.response)
    end
end)
```

### Chat Conversations

#### `Ollama.Chat(model, messages, callback)`
Conduct a conversation with context.

**Parameters:**
- `model` (string): Model name
- `messages` (table): Array of message objects with `role` and `content` fields
- `callback` (function): Callback function `function(err, data)`

**Message format:**
```lua
{
    {role = "system", content = "You are a helpful assistant"},
    {role = "user", content = "Hello!"},
    {role = "assistant", content = "Hi there!"},
    {role = "user", content = "How are you?"}
}
```

**Callback data structure:**
```lua
{
    content = "AI response content",
    role = "assistant",
    model = "llama2:latest"
}
```

**Example:**
```lua
local conversation = {
    {role = "system", content = "You are a helpful assistant for Garry's Mod players."},
    {role = "user", content = "How do I create a basic prop in GMod?"},
}
Ollama.Chat("llama2", conversation, function(err, data)
    if err then
        print("Error: " .. err)
    else
        print("Chat Response from " .. data.model .. " (" .. data.role .. "): " .. data.content)
    end
end)
```

### Model Management

#### `Ollama.ListModels(callback)`
List all available models.

**Callback data structure:**
```lua
{
    {
        name = "llama2:latest",
        modified_at = "2023-12-01T12:00:00Z",
        size = 3825819519,
        digest = "sha256:..."
    },
    -- ... more models
}
```

**Example:**
```lua
Ollama.ListModels(function(err, models)
    if err then
        print("Error: " .. err)
    else
        print("Available models:")
        for i, model in ipairs(models) do
            local size_mb = math.floor(model.size / 1024 / 1024)
            print("  " .. i .. ". " .. model.name .. " (Size: " .. size_mb .. " MB)")
        end
    end
end)
```

#### `Ollama.GetModelInfo(model, callback)`
Get detailed information about a specific model.

**Callback data structure:**
```lua
{
    license = "Model license text",
    modelfile = "Modelfile content",
    parameters = "Model parameters",
    template = "Prompt template"
}
```

**Example:**
```lua
Ollama.GetModelInfo("llama2", function(err, data)
    if err then
        print("Error: " .. err)
    else
        print("Model Info for llama2:")
        print("  License: " .. (data.license ~= "" and data.license or "N/A"))
        print("  Parameters: " .. (data.parameters ~= "" and data.parameters or "N/A"))
        print("  Template length: " .. string.len(data.template))
    end
end)
```

#### `Ollama.IsModelAvailable(model, callback)`
Check if a specific model is available.

**Example:**
```lua
Ollama.IsModelAvailable("llama2", function(err, is_available)
    if err then
        print("Error: " .. err)
    else
        print("llama2 is " .. (is_available and "available" or "not available"))
    end
end)
```

#### `Ollama.GetRunningModels(callback)`
List models currently loaded into memory.

**Callback data structure:**
```lua
{
    {
        name = "llama2:latest",
        model = "llama2:latest",
        size = 3825819519,
        digest = "sha256:...",
        expires_at = "2023-12-01T12:05:00Z", -- optional
        size_vram = 3825819519 -- optional
    },
    -- ... more running models
}
```

**Example:**
```lua
Ollama.GetRunningModels(function(err, models)
    if err then
        print("Error: " .. err)
    else
        if #models > 0 then
            print("Currently running models:")
            for i, model in ipairs(models) do
                local size_mb = math.floor(model.size / 1024 / 1024)
                local vram_mb = model.size_vram and math.floor(model.size_vram / 1024 / 1024) or 0
                print("  " .. i .. ". " .. model.name)
                print("     Size: " .. size_mb .. " MB (VRAM: " .. vram_mb .. " MB)")
                if model.expires_at then
                    print("     Expires: " .. model.expires_at)
                end
            end
        else
            print("No models currently running")
        end
    end
end)
```

### Embeddings

#### `Ollama.GenerateEmbeddings(model, input, callback)`
Generate embeddings from text input.

**Parameters:**
- `model` (string): Model name (e.g., "all-minilm")
- `input` (string or table): Single string or array of strings
- `callback` (function): Callback function `function(err, data)`

**Callback data structure:**
```lua
{
    model = "all-minilm:latest",
    embeddings = {
        {0.1, -0.2, 0.3, ...}, -- First embedding vector
        {0.2, -0.1, 0.4, ...}, -- Second embedding vector (if multiple inputs)
        -- ... more vectors
    }
}
```

**Single text example:**
```lua
Ollama.GenerateEmbeddings("all-minilm", "Why is the sky blue?", function(err, data)
    if err then
        print("Error: " .. err)
    else
        print("Generated embeddings for model: " .. data.model)
        print("Number of embedding vectors: " .. #data.embeddings)
        if #data.embeddings > 0 then
            print("First embedding dimensions: " .. #data.embeddings[1])
        end
    end
end)
```

**Multiple texts example:**
```lua
local texts = {
    "Hello world",
    "How are you today?",
    "Garry's Mod is fun"
}
Ollama.GenerateEmbeddings("all-minilm", texts, function(err, data)
    if err then
        print("Error: " .. err)
    else
        print("Generated " .. #data.embeddings .. " embedding vectors")

        -- Calculate similarity between first two embeddings
        if #data.embeddings >= 2 then
            local function dot_product(a, b)
                local sum = 0
                for i = 1, math.min(#a, #b) do
                    sum = sum + (a[i] * b[i])
                end
                return sum
            end

            local function magnitude(vec)
                local sum = 0
                for i = 1, #vec do
                    sum = sum + (vec[i] * vec[i])
                end
                return math.sqrt(sum)
            end

            local emb1, emb2 = data.embeddings[1], data.embeddings[2]
            local similarity = dot_product(emb1, emb2) / (magnitude(emb1) * magnitude(emb2))
            print("Similarity between first two texts: " .. string.format("%.4f", similarity))
        end
    end
end)
```

## Chat Commands Integration

```lua
hook.Add("PlayerSay", "OllamaChat", function(ply, text)
    if string.StartWith(text, "!ai ") then
        local prompt = string.sub(text, 5)

        Ollama.Generate("llama2", prompt, nil, function(err, data)
            if err then
                ply:ChatPrint("AI Error: " .. err)
            else
                ply:ChatPrint("AI: " .. data.response)
            end
        end)

        return ""
    elseif string.StartWith(text, "!models") then
        Ollama.ListModels(function(err, models)
            if err then
                ply:ChatPrint("Error: " .. err)
            else
                if models and #models > 0 then
                    ply:ChatPrint("Available models:")
                    for i, model in ipairs(models) do
                        if i <= 5 then -- Limit to first 5 to avoid spam
                            local size_mb = math.floor(model.size / 1024 / 1024)
                            ply:ChatPrint("  " .. model.name .. " (" .. size_mb .. "MB)")
                        end
                    end
                    if #models > 5 then
                        ply:ChatPrint("  ... and " .. (#models - 5) .. " more")
                    end
                else
                    ply:ChatPrint("No models available")
                end
            end
        end)

        return ""
    elseif string.StartWith(text, "!running") then
        Ollama.GetRunningModels(function(err, models)
            if err then
                ply:ChatPrint("Error: " .. err)
            else
                if #models > 0 then
                    ply:ChatPrint("Currently running models:")
                    for i, model in ipairs(models) do
                        local size_mb = math.floor(model.size / 1024 / 1024)
                        local vram_mb = model.size_vram and math.floor(model.size_vram / 1024 / 1024) or 0
                        ply:ChatPrint("  " .. model.name .. " (" .. size_mb .. "MB, VRAM: " .. vram_mb .. "MB)")
                    end
                else
                    ply:ChatPrint("No models currently running")
                end
            end
        end)

        return ""
    elseif string.StartWith(text, "!embed ") then
        local input_text = string.sub(text, 8)

        Ollama.GenerateEmbeddings("all-minilm", input_text, function(err, data)
            if err then
                ply:ChatPrint("Embedding Error: " .. err)
            else
                ply:ChatPrint("Generated embedding with " .. #data.embeddings[1] .. " dimensions")
                ply:ChatPrint("Model: " .. data.model)
            end
        end)

        return ""
    end
end)
```

## Advanced Conversation System

```lua
local function startConversation()
    local history = {
        {role = "system", content = "You are an AI assistant in Garry's Mod. Help players with game-related questions."}
    }

    local function addMessage(role, content)
        table.insert(history, {role = role, content = content})
    end

    local function sendMessage(message)
        addMessage("user", message)

        Ollama.Chat("llama2", history, function(err, data)
            if err then
                print("Conversation Error: " .. err)
            else
                addMessage("assistant", data.content)
                print("AI: " .. data.content)
            end
        end)
    end

    -- Example conversation
    sendMessage("What's the best way to build in GMod?")
end
```

## Error Handling

All callbacks follow the error-first pattern:
- First parameter is the error (string or nil)
- Second parameter is the result data (structured table)

```lua
Ollama.Generate("model", "prompt", nil, function(err, data)
    if err then
        -- Handle error
        print("Something went wrong: " .. err)
    else
        -- Handle success
        print("Got response: " .. data.response)
        print("From model: " .. data.model)
    end
end)
```

## Troubleshooting

1. **"Ollama request failed"**: Check if Ollama is running and accessible
2. **Module not loading**: Ensure correct binary name and placement
3. **Slow responses**: Large models take time
4. **Memory issues**: Monitor model sizes and server resources
5. **Generation fails**: Make sure you have installed the model you're trying to use

## License

MIT License - see LICENSE file for details.
