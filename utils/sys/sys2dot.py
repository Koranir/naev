#!/usr/bin/env python3

if __name__ != '__main__':
   raise Exception('This module is only intended to be used as main.')


from sys import argv, stderr

from sys2graph import xml_files_to_graph


anbh = [ 'ngc11935', 'ngc5483', 'ngc7078', 'ngc7533', 'octavian',
   'copernicus', 'ngc13674', 'ngc1562', 'ngc2601', ]

virtual_edges=[
   ('flow', 'basel', 2, 1),
   ('deneb', 'booster', 1.5, 1),
   ('ngc4746', 'logania', 1, 1),
]

prv = None
prvj = None
bhl = 1.0
for j, i in enumerate(anbh):
   if prv is None:
      prv = i
   else:
      if prvj is not None:
         virtual_edges.append(('_'+str(prvj),    '_'+str(j), bhl, 100))
      prvj = j
      virtual_edges.append(('anubis_black_hole', '_'+str(j), bhl, 100))
      virtual_edges.append(('_'+str(j),                 prv, bhl, 100))
      virtual_edges.append(('_'+str(j),                   i, bhl, 100))
      prv = None

if prv is not None:
   virtual_edges.append((prv,                           i, bhl, 100))
   virtual_edges.append(('_'+str(prvj),   '_'+str(prvj+2), bhl, 100))
   virtual_edges.append(('_'+str(prvj+2),             prv, bhl, 100))
   virtual_edges.append(('_'+str(prvj+2),      '_'+str(1), bhl, 100))

heavy_virtual_edges=[
   #('akodu', 'kenvis'),
   #('tau_ceti', 'sigur'), ('tepvin', 'carrza'),
   ('thirty_stars', 'thorndyke'),
   ('herakin', 'duros'), ('rauthia', 'tide'),
   ('hekaras', 'eneguoz'), ('seifer', 'rei'),
   ('basel', 'octantis'), ('sagittarius', 'baitas'),
   ('baitas', 'tasopa'),
   ('percival', 'jommel'), ('basel', 'octantis'),
   ('flow', 'katami'), ('nava', 'flow'),
   ('katami', 'eisenhorn'), ('vean', 'basel'),
   ('alpha_centauri', 'tasopa'),('syndania', 'padonia'),
   ('veses', 'protera'), ('syndania', 'stint'),
   ('sagittarius', 'alpha_centauri'), ('protera', 'scholzs_star'),
   ('ngc18451', 'felzen'), ('ngc6057', 'xeric'),
   ('kiwi', 'suna'), ('ngc1098', 'westhaven'),
   ('ngc7061', 'kansas'), ('niger', 'kyo'),
   ('willow', 'palovi'), ('margarita', 'narousse'),
   ('porro', 'modus_manis'), ('suna', 'vanir'),
   ('tobanna', 'brumeria'),('rotide', 'tide'),
   ('padonia', 'basel'), ('ogat', 'wochii'),
   ('griffin', 'pastor'), ('ngc2948', 'ngc9017'),
   ('ngc4131', 'neexi'), ('c59', 'c14'),
   ('c43', 'c28'), ('levo', 'qellan'),
   ('nixon', 'gyrios'), ('suk', 'oxuram'),
   ('defa', 'taiomi'), ('titus', 'solene'), ('titus', 'diadem'),
   ('pike', 'kraft'), ('undergate', 'ulysses'),
   ('ngc20489', 'monogram'), ('anrique', 'adraia'),
   ('andee', 'chraan'), ('trohem', 'tepdania'),
   ('ngc14479', 'zintar'), ('pudas', 'fried'),
   ('blunderbuss', 'darkstone'), ('ekkodu', 'tarsus'),
   ('ivella', 'jommel'), ('starlight_end', 'possum'),
   ('ngc8338', 'unicorn'), ('ngc22375', 'undergate'),
]

def main( args, fixed_pos = False, color = False ):
   V, E, pos, tl, colors = xml_files_to_graph(args, color)
   print('graph g{')
   print('\tepsilon=0.0000001')
   print('\tmaxiter=1000')
   #print('\tDamping=0.5')
   #print('\tmode=ipsep')

   # 1inch=72pt
   if fixed_pos:
      print('\tgraph [overlap=true]')
      factor = 0.7
   else:
      print('\tgraph [overlap=false]')  #'\toverlap=voronoi'
      factor = 0.7

   print('\tinputscale=72')
   print('\tnotranslate=true') # don't make upper left at 0,0
   print('\tnode[fixedsize=true,shape=circle,color=white,fillcolor=grey,style="filled"]')
   reflen = 0.5
   print('\tnode[width=0.5]')
   print('\tedge[len='+str(reflen)+']')
   print('\tedge[weight=100]')

   if fixed_pos:
      print('\tnode[pin=true]')

   virt_v = set()
   for (f, t, l, w) in virtual_edges:
      virt_v.update({f, t})

   if not fixed_pos:
      for i in virt_v:
         if i not in V:
            print('\t"'+i+'" [label="",style=invis]')

   for i in V:
      if i[0] == '_' and fixed_pos:
         continue
      # Don't include disconnected systems
      if E[i] != [] or fixed_pos:
         s = '\t"'+i+'" ['
         if i[0] != '_':
            (x, y) = pos[i]
            x = round(float(x)*factor, 9)
            y = round(float(y)*factor, 9)
            s += 'pos="'+str(x)+','+str(y)+('!' if fixed_pos else '')+'";'
         label = V[i]
         for t in [('-','- '), (' ','\\n'), ('Test\\nof','Test of')]:
            label = label.replace(*t)
         s += 'label="' + label + '"'

         if color:
            cols = [int(255.0*(f/3.0+2.0/3.0)) for f in colors[i]]
            rgb = ''.join([('0'+(hex(v)[2:]))[-2:] for v in cols])
            s += ';fillcolor="#'+rgb+'"'

         if i == 'sol':
            s += ';color=red'

         print(s + ']')
         for dst, hid in E[i]:
            suff = []
            if i in tl and dst in tl:
               suff.extend(['style=bold', 'penwidth=4.0'])
            elif hid:
               suff.extend(['style=dotted', 'penwidth=2.5'])

            suff = '[' + ';'.join(suff) + ']' if suff != [] else ''

            oneway = i not in map(lambda t:t[0], E[dst])
            edge = '->' if oneway else '--'
            if oneway or i<dst:
               print('"'.join(['\t', i, edge, dst, suff]))

   if not fixed_pos:
      print('\tedge[len=' + str(reflen) + ';weight=100]')
      print('\tedge[style="dashed";color=grey;pendwidth=1.5]')
      for (f, t) in heavy_virtual_edges:
         inv = '[style=invis]' if (f in virt_v or t in virt_v) else ''
         print('\t"'+f+'"--"'+t+'"'+inv)

      for (f, t, l, w) in virtual_edges:
         inv = ', style=invis' if (f in virt_v or t in virt_v) else ''
         print('\t"'+f+'"--"'+t+'" [len='+str(l*reflen)+',weight='+str(w)+inv+']')

   print('}')

if __name__ == '__main__':
   if '-h' in argv[1:] or '--help' in argv[1:] or len(argv)<2:
      print('usage: ', argv[0], '[-c]', '[-k]', '<sys1.xml>', '...')
      print('Outputs the graph in dot format.')
      print('If -c is set, use faction colors (slower).')
      print('If -k is set, the nodes have the keep_position marker.')
      print('Examples:')
      print('  > ./utils/sys2dot.py dat/ssys/*.xml -k | neato -Tpng > before.png')
      print('  > ./utils/sys2dot.py dat/ssys/*.xml | neato -Tpng > after.png')
      print('  > ./utils/sys2dot.py dat/ssys/*.xml | neato | tee after.dot |  ./utils/sys/dot2sys.py')
      print('  > display before.png after.png')
   else:
      if keep := '-k' in argv:
         argv.remove('-k')

      if color := '-c' in argv:
         argv.remove('-c')

      if (ign := [f for f in argv[1:] if not f.endswith('.xml')]) != []:
         stderr.write('Ignored: "' + '", "'.join(ign) + '"\n')

      main([f for f in argv[1:] if f.endswith('.xml')], keep, color)
