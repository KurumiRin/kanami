--!strict
-- Leader key: press leader first, then a few keys in turn to trigger an action.
--
-- Config:
--   [[rule]]
--   from = "right_option"
--   call = "leader"
--
-- Define sequences in the script; a value can be a key expression (tap) or a function:
--   leader_map({
--       ["g h"] = function() kanami.run("open https://github.com") end,
--       ["w q"] = "cmd+q",
--   })
--
-- Esc or a timeout (1 second by default) cancels; so does a key that is in no sequence.

local TIMEOUT = 1000
local MODIFIERS = {
    left_control = true, right_control = true, left_shift = true, right_shift = true,
    left_option = true, right_option = true, left_command = true, right_command = true, fn = true,
}

local sequences: { [string]: any } = {}
local active = false
local typed: { string } = {}
local timer: number? = nil

function leader_map(map: { [string]: any })
    for seq, action in map do
        sequences[seq] = action
    end
end

local function stop()
    active = false
    typed = {}
    if timer then
        kanami.cancel(timer)
        timer = nil
    end
end

local function restart_timer()
    if timer then
        kanami.cancel(timer)
    end
    timer = kanami.after(TIMEOUT, stop)
end

local function perform(action: any)
    if type(action) == "function" then
        action()
    else
        kanami.tap(action)
    end
end

function leader()
    active = true
    typed = {}
    restart_timer()
end

kanami.on_key(function(e)
    if not active or MODIFIERS[e.key] then
        return false
    end
    if e.state ~= "down" then
        return true
    end
    if e.key == "escape" then
        stop()
        return true
    end
    table.insert(typed, e.key)
    local seq = table.concat(typed, " ")
    local action = sequences[seq]
    if action ~= nil then
        stop()
        perform(action)
        return true
    end
    for s in sequences do
        if string.sub(s, 1, #seq + 1) == seq .. " " then
            restart_timer()
            return true
        end
    end
    stop()
    return true
end)
