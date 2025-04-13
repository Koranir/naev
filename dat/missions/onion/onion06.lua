--[[
<?xml version='1.0' encoding='utf8'?>
<mission name="Onion Society 06">
 <unique />
 <priority>3</priority>
 <chance>100</chance>
 <location>Bar</location>
 <done>Onion Society 05</done>
 <cond>
   local c = spob.cur()
   local f = c:faction()
   if not f or not f:tags("generic") then
      return false
   end
   return true
 </cond>
 <notes>
  <campaign>Onion Society</campaign>
 </notes>
</mission>
--]]
--[[
   Onion 06

   Emergency Conclave timeTODO explanation
--]]
local fmt = require "format"
local vn = require "vn"
local onion = require "common.onion"
local love_shaders = require "love_shaders"
local tut = require "common.tutorial"
local lg = require "love.graphics"

local title = _("Emergency Conclave")

-- Create the mission
function create()
   misn.finish() -- Not ready yet

   local prt = love_shaders.shaderimage2canvas( love_shaders.hologram(), onion.img_onion() )
   misn.setNPC( _("Onion Society Conclave"), prt.t.tex, _([[It seems like your Ship AI is able to set you up with a connection to the Onion Society Conclave.]]) )
   misn.setReward(_("???") )
   misn.setTitle( title )
   misn.setDesc(_([[Join the emergency Onion Society conclave.]]))
end

function accept ()
   local accepted = false

   vn.clear()
   vn.scene()
   local sai = vn.newCharacter( tut.vn_shipai() )
   vn.transition( tut.shipai.transition )
   vn.na(fmt.f(_([[Your ship AI {shipai} materializes in front of you.]]),
      {shipai=tut.ainame()}))
   -- Is it truly your SAI, or is it... DOG!??!
   sai(_([["I'll spare you the details, but I believe it may be of your interest to join an emergency Onion Society conclave to keep an eye out on l337_b01."
The hologram slightly flickers.
"I can arrange a connection if you so desire."]]))
   vn.label("menu")
   vn.menu{
      {_([[Establish the connection.]]), "accept"},
      {_([[Maybe later.]]), "decline"},
      {_([[How can you do this?]]), "explain"},
   }

   vn.label("decline")
   vn.na(_([[You think it's maybe best to not join a risky Onion Society Conclave. Maybe later.]]))
   vn.done( tut.shipai.transition )

   vn.label("explain")
   sai(_([["I hope you are not offended, I was able to record the protocol last time you joined, and should be able to run the connections synchronously to join like before."]]))
   vn.jump("menu")

   vn.label("accept")
   vn.scene()
   local bg_cyberspace = love_shaders.shader2canvas( love_shaders.cyberspace() )
   vn.setBackground( function ()
      lg.setColour( 1, 1, 1, 1 )
      bg_cyberspace:draw( 0, 0 )
   end )
   vn.transition("fadeup")
   vn.na(_([[You accept and strap into a full sensorial holodeck. Once again you are thrust into an uncanny feeling as you plunge into the Nexus.]]))
   if var.peek( "nexus_sickness" ) then
      vn.na(_([[At least this time you seem to have much less Nexus syndrome effects.]]))
   end

   vn.scene()
   local offset = 1/7
   local l337 = vn.newCharacter( onion.vn_nexus_l337b01{pos=1*offset} )
   local underworlder = vn.newCharacter( onion.vn_nexus_underworlder{pos=2*offset, flip=true} )
   local puppet = vn.newCharacter( onion.vn_nexus_notasockpuppet{pos=3*offset, flip=true} )
   -- Trixie would be here, but we keep it empty on purpose
   local dog = vn.newCharacter( onion.vn_nexus_dog{pos=5*offset, flip=false} )
   local lonewolf4 = vn.newCharacter( onion.vn_nexus_lonewolf4{pos=6*offset, flip=false} )
   vn.music( onion.loops.circus )
   vn.transition()
   l337(_([[]]))
   underworlder(_([[]]))
   puppet(_([[]]))
   dog(_([[]]))
   lonewolf4(_([[]]))

   vn.done("electric")
   vn.run()

   if not accepted then return end

   misn.finish(true)
end
