#!/usr/bin/env python3


from os.path import basename
from geometry import transf, vec
from ssys import nam2base, starmap, fil_ET, spob_fil, vec_to_element, vec_from_element
from math import sin, pi
from minimize_angle_stretch import relax_dir

sm = starmap()


def _key(v):
   acc = 0
   if v[1] < 0:
      acc += 2
      v = -v
   return acc + (2 - v[0])

def mk_p(L):
   pi = [n for n, _k in sorted(enumerate(L), key = lambda t: (_key(t[1]), t[0]))]
   where = pi.index(0)
   return pi[where:] + pi[:where]

def sys_relax( sys, quiet = True, graph = False ):
   p = fil_ET(sys)
   T = p.getroot()
   myname = nam2base(T.attrib['name'])

   mapvs, sysvs, names = [], [], []
   for f in T.findall('./jumps/jump'):
      dst = nam2base(f.attrib['target'])
      e= f.find('pos')
      # should we require 'was_auto' ?
      if e is not None and 'was_auto' in e.attrib:
         names.append(dst)
         mapvs.append((sm[dst] - sm[myname]).normalize())
         sysvs.append(vec_from_element(e).normalize())

   if names != []:
      nop = lambda v: v
      flip = nop
      pi1, pi2 = mk_p(mapvs), mk_p(sysvs)
      # more than max possible cost <= 2.0
      uf_cost = 3.0
      eps = 0.0001
      out = sys.replace('.xml', '') if graph else None

      if pi1 == pi2:
         # same permutation -> everything is fine
         pass
      else:
         flip = lambda sysv: vec(-sysv[0], sysv[1])
         if pi1 == pi2[:1] + pi2[1:][::-1]:
            stderr.write('\033[32m"' + basename(sys) + '" flipped !\033[0m\n')
         else:
            stderr.write('\033[33m"' + basename(sys) + '" crossed : ')
            pi1 = [names[i] for i in pi1]
            pi2 = [names[i] for i in pi2]
            stderr.write(', '.join(pi1) + ' -> ' + ', '.join(pi2) + '\033[0m')
            stderr.write('\n' if not quiet else '')
            outu = None if out is None else (out+'_u')
            uf_alpha, uf_cost = relax_dir(sysvs, mapvs, eps = eps/10.0, debug = outu, quiet = quiet)

      alpha, cost = relax_dir([flip(v) for v in sysvs], mapvs, eps = eps/10.0, debug = out, quiet = quiet )
      if uf_cost < 3.0:
         if cost > uf_cost:
            alpha, flip = uf_alpha, nop
            stderr.write(' \033[33m[better unflipped]\033[0m\n')
         else:
            stderr.write(' \033[33m[better flipped]\033[0m\n')

      if abs(alpha) > eps or flip != nop:
         func = lambda x: flip(x).rotate(alpha)
         for e in T.findall('./spobs/spob'):
            spfil = spob_fil(nam2base(e.text))
            p2 = fil_ET(spfil)
            f = p2.getroot().find('pos')
            vec_to_element(f, func(vec_from_element(f)))
            p2.write(spfil)

         for i in [ './jumps/jump/pos', './asteroids/asteroid/pos',
            './waypoints/waypoint']:
            for e in T.findall(i):
               vec_to_element(e, func(vec_from_element(e)))
         p.write(sys)
         return True
   return False

if __name__ == '__main__':
   from sys import argv, exit, stderr
   from os.path import basename
   args = argv[1:]

   if '-h' in args or '--help' in args or args == []:
      stderr.write('usage:  ' + basename(argv[0]) + '  [-v|-g]  <file1> ..\n')
      stderr.write('  Relaxes its input xml ssys files.\n')
      stderr.write('  If -v is set, display information.\n')
      stderr.write('  If -g is set, outputs the cost graph.\n')
      exit(0)

   if verbose:= '-v' in args:
      args.remove('-v')

   if graph:= '-g' in args:
      args.remove('-g')

   for i in args:
      if sys_relax(i, quiet = not verbose, graph = graph):
         print(i)
