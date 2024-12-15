import numpy as np
import matplotlib.pyplot as plt

DELTA_T = 10

# GM = 6.67408e-11 * 1.99855e30
# RAD = 1.496e11
# D_PHI = 1.99086e-7
# D_RAD = 0.0
# print(GM/(RAD*RAD), RAD*D_PHI*D_PHI, RAD*D_PHI*D_PHI - GM/(RAD*RAD))

GM = 1.0
RAD = 60
#D_PHI = 6.3e-3
#D_RAD = 1e-1

#FACTOR = 1.0

#D_PHI = FACTOR * np.sqrt(GM/(RAD*RAD*RAD))

#print(GM/(RAD*RAD), RAD*D_PHI*D_PHI, RAD*D_PHI*D_PHI - GM/(RAD*RAD))

class Planet:
    def __init__(self, rad, d_phi):
        self._rad_initial = rad
        self._d_phi_initial = d_phi
        self.reset()

    def reset(self):
        self._phi = 0#np.random.random() * 2*np.pi
        self._rad = self._rad_initial
        self._d_rad = 0.0
        self._d_phi = self._d_phi_initial

    @classmethod
    def from_vis_viva(cls, rad, semi_major_norm):
        print("planet")
        semi_major = rad * semi_major_norm
        v = np.sqrt(GM*np.abs((2.0/rad) - (1.0/semi_major)))
        d_phi = v / (rad)
        print("d_phi:", d_phi)
        return cls(rad, d_phi)

    def step_rad(self, n):
        delta_t = DELTA_T / n
        for _ in range(n):
            dd_rad = self._rad * self._d_phi * self._d_phi - GM/(self._rad*self._rad)
            self._d_rad += dd_rad * delta_t
            self._rad += self._d_rad * delta_t

    def step_phi(self, n):
        delta_t = DELTA_T / n
        for _ in range(n):
            dd_phi = - 2 * self._d_rad * self._d_phi / self._rad
            self._d_phi += dd_phi * delta_t
            self._phi += self._d_phi * delta_t


    def process(self):
        #print("process")
        if self._rad <= 0:
            return (0, 0)


        self.step_rad(20)
        self.step_phi(20)
        #print(self._rad, self._phi)# / (2.0 * np.pi) * 24)

        if self._rad <= 0:
            return (0, 0)

        #self._phi %= 2.0*np.pi

        #print(np.array([self._rad, self._phi]))
        return np.array([self._rad, self._phi])

    def process_int(self):
        phi = int(np.floor(self._phi / (2.0 * np.pi) * 24))
        rad = int(np.floor(self._rad))

        while True:
            _rad, _phi = self.process()
            _phi = int(np.floor(_phi / (2.0 * np.pi) * 24))
            _rad = int(np.floor(_rad))

            if _phi != phi:
                return [_rad, _phi]


fig, ax = plt.subplots(subplot_kw={'projection': 'polar'})
#planet = Planet(RAD, D_PHI)
planet = Planet.from_vis_viva(RAD, 10.0)
print("planet created")
polar = np.stack([planet.process_int() for _ in range(24)], axis=1)
rad = polar[0, :] #/ 2
phi = polar[1, :] / 24 * 2.0*np.pi
ax.plot(phi, rad, 'o')

planet.reset()
polar = np.stack([planet.process() for _ in range(50000)], axis=1)
rad = polar[0, :] #/ 2
phi = polar[1, :] # 24 * 2.0*np.pi
ax.plot(phi, rad, '-')

print(rad.max(), rad.min(), rad.argmin())
print(phi.max(), phi.min(), phi.max() / 2.0 / np.pi)

plt.show()
