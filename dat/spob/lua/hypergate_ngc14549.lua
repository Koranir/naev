local hypergate = require "spob.lua.lib.hypergate"
hypergate.setup{
   basecol = { 0.4, 0.2, 0.2 }, -- Pirate
   cost_mod = {
      [95]  = 0.1,
      [80]  = 0.3,
      [60]  = 0.5,
      [40]  = 0.8,
      [20]  = 1,
      [0]   = 1.5,
      [-20] = 2,
      [-50] = 4,
   },
}
