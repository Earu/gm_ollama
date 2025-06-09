-- Example usage of the gm_ollama module
-- Place this in your GMod addon's lua/autorun folder

-- Configure Ollama connection (optional - defaults to localhost:11434)
Ollama.SetConfig("http://localhost:11434", 30) -- URL and timeout in seconds

-- Check if Ollama is running
if Ollama.IsRunning() then
    print("Ollama is running!")
else
    print("Ollama is not running or not accessible")
    return
end

print("=== Generation ===")
-- Note: "llama2" will automatically become "llama2:latest"
Ollama.Generate("llama2", "Tell me a short joke about programming", nil, function(err, data)
    if err then
        print("Error: " .. err)
    else
        print("Response from " .. data.model .. ": " .. data.response)
    end
end)

print("=== Generation with System Prompt ===")
Ollama.Generate("llama2", "Explain quantum physics", "You are a helpful physics teacher. Keep explanations simple.", function(err, data)
    if err then
        print("Error: " .. err)
    else
        print("Physics Response: " .. data.response)
    end
end)

-- Chat example
print("=== Asynchronous Chat ===")
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

-- List available models
print("=== List Models ===")
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

-- Check if a specific model is available
print("=== Check Model Availability ===")
Ollama.IsModelAvailable("llama2", function(err, is_available)
    if err then
        print("Error: " .. err)
    else
        print("llama2 is " .. (is_available and "available" or "not available"))
    end
end)

-- Get model information
print("=== Model Information ===")
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

-- Get running models
print("=== Get Running Models ===")
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

-- Generate embeddings for a single text
print("=== Generate Embeddings (Single Text) ===")
Ollama.GenerateEmbeddings("all-minilm", "Why is the sky blue?", function(err, data)
    if err then
        print("Error: " .. err)
    else
        print("Generated embeddings for model: " .. data.model)
        print("Number of embedding vectors: " .. #data.embeddings)
        if #data.embeddings > 0 then
            print("First embedding dimensions: " .. #data.embeddings[1])
            print("First few values: " .. table.concat({
                data.embeddings[1][1] or 0,
                data.embeddings[1][2] or 0,
                data.embeddings[1][3] or 0
            }, ", ") .. "...")
        end
    end
end)

-- Generate embeddings for multiple texts
print("=== Generate Embeddings (Multiple Texts) ===")
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
        for i, embedding in ipairs(data.embeddings) do
            print("Text " .. i .. " embedding dimensions: " .. #embedding)
        end

        -- Example: Calculate similarity between first two embeddings (cosine similarity)
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

-- Example of a more complex conversation system
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

    -- You could hook this up to chat commands, GUI, etc.
end

-- Example chat command integration
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
    elseif string.StartWith(text, "!check ") then
        local model_name = string.sub(text, 8)

        Ollama.IsModelAvailable(model_name, function(err, data)
            if err then
                ply:ChatPrint("Error: " .. err)
            else
                ply:ChatPrint(model_name .. " is " .. (data.available and "available" or "not available"))
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
    end
end)

-- Uncomment to test conversation system
-- startConversation()

print("Ollama examples loaded! All operations are asynchronous - check console for responses.")