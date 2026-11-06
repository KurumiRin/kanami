--!strict
-- Tap dance: pressing the same key a different number of times triggers different actions.
--
--   tap_dance("f13", { "escape", "cmd+q" })              -- once is Esc, twice is ⌘Q
--   tap_dance("semicolon", { "semicolon", "shift+semicolon" }, 150) -- twice types a colon, with a custom interval (milliseconds)
--
-- An action can be a key expression (tap) or a function. It fires immediately once the press count
-- reaches the number of actions; otherwise it fires after the interval ends following the last press.
-- Pressing another key in between first fires the action for the current count.

type Dance = { actions: { any }, timeout: number }
type Current = { key: string, count: number, timer: number? }

local dances: { [string]: Dance } = {}
local current: Current? = nil

function tap_dance(key: string, actions: { any }, timeout: number?)
    dances[key] = { actions = actions, timeout = timeout or 200 }
end

local function finish()
    local c = current
    if c == nil then
        return
    end
    current = nil
    if c.timer then
        kanami.cancel(c.timer)
    end
    local action = dances[c.key].actions[c.count]
    if type(action) == "function" then
        action()
    else
        kanami.tap(action)
    end
end

kanami.on_key(function(e)
    local dance = dances[e.key]
    if current and e.state == "down" and e.key ~= current.key then
        finish()
    end
    if dance == nil then
        return false
    end
    if e.state ~= "down" then
        return true
    end
    local c = current
    if c and c.key == e.key then
        c.count += 1
        if c.timer then
            kanami.cancel(c.timer)
        end
    else
        c = { key = e.key, count = 1 }
        current = c
    end
    assert(c)
    if c.count >= #dance.actions then
        finish()
    else
        c.timer = kanami.after(dance.timeout, finish)
    end
    return true
end)
