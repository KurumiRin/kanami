--!strict
-- Text expansion: typing an abbreviation replaces it with the full text.
--
--   expand(";sig", "Best\nregards")
--   expand(";mail", "me@example.com")
--
-- Only active on plain keyboard layouts (no expansion while an input method such as Chinese is composing).
-- ⌘, ⌃ and ⌥ combos, arrow keys, Return and the like clear the typed buffer.

local CHARS: { [string]: string } = {
    space = " ", minus = "-", equal = "=", left_bracket = "[", right_bracket = "]",
    backslash = "\\", semicolon = ";", quote = "'", comma = ",", period = ".",
    slash = "/", grave = "`",
}
for c in string.gmatch("abcdefghijklmnopqrstuvwxyz0123456789", ".") do
    CHARS[c] = c
end
local MODIFIERS = {
    left_control = true, right_control = true, left_shift = true, right_shift = true,
    left_option = true, right_option = true, left_command = true, right_command = true, fn = true,
}

local expansions: { [string]: string } = {}
local longest = 0
local buffer = ""

function expand(abbreviation: string, text: string)
    expansions[abbreviation] = text
    longest = math.max(longest, #abbreviation)
end

local function plain_layout(): boolean
    local source = kanami.input_source()
    return source == nil or string.find(source, "keylayout", 1, true) ~= nil
end

kanami.on_key(function(e)
    if e.state ~= "down" or MODIFIERS[e.key] then
        return false
    end
    if e.key == "backspace" then
        buffer = string.sub(buffer, 1, -2)
        return false
    end
    local char = CHARS[e.key]
    local m = e.mods
    if char == nil or m.cmd or m.ctrl or m.opt or m.shift or not plain_layout() then
        buffer = ""
        return false
    end
    buffer = string.sub(buffer .. char, -longest)
    for abbreviation, text in expansions do
        if #buffer >= #abbreviation and string.sub(buffer, -#abbreviation) == abbreviation then
            buffer = ""
            -- The last character was swallowed; delete the part that was already typed
            for _ = 1, #abbreviation - 1 do
                kanami.tap("backspace")
            end
            kanami.type(text)
            return true
        end
    end
    return false
end)
