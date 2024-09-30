local var = {}

local function _v( tbl )
   if #tbl==1 then
      return function () return tbl[1].s end
   end

   table.sort( tbl, function ( a, b )
      return a.w > b.w
   end )
   local wmax = 0
   for k,v in ipairs(tbl) do
      wmax = wmax+v.w
      if not v.s then
         warn(_("Variant has s==nil!"))
      end
   end
   return function ()
      local r = rnd.rnd()*wmax
      local w = 0
      for k,v in ipairs(tbl) do
         w = w+v.w
         if r <= w then
            return v.s
         end
      end
   end
end

-- Yacht
var.llama = _v{
   { w=1,    s=ship.get("Llama") },
   { w=0.05, s=ship.get("Llama Voyager") },
}
var.gawain = _v{
   { w=1,    s=ship.get("Gawain") },
}
-- Courier
var.koala = _v{
   { w=1,    s=ship.get("Koala") },
   { w=0.05, s=ship.get("Koala Armoured") },
}
var.quicksilver = _v{
   { w=1,    s=ship.get("Quicksilver") },
   { w=0.05, s=ship.get("Quicksilver Mercury") },
}
-- Freighter
var.mule = _v{
   { w=1,    s=ship.get("Mule") },
   { w=0.05, s=ship.get("Mule Hardhat") },
}
-- Armoured Transport
var.rhino = _v{
   { w=1,    s=ship.get("Rhino") },
}
-- Scout
var.schroedinger = _v{
   { w=1,    s=ship.get("Schroedinger") },
}
-- Destroyer
var.starbridge = _v{
   { w=1,    s=ship.get("Starbridge") },
   { w=0.05, s=ship.get("Starbridge Sigma") },
}

return var