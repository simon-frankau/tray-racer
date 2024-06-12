import matplotlib
import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np

matplotlib.use('agg')

data_0_01 = np.genfromtxt('ss-0.01.csv', delimiter = ',', names = True)
data_0_02 = np.genfromtxt('ss-0.02.csv', delimiter = ',', names = True)
data_0_04 = np.genfromtxt('ss-0.04.csv', delimiter = ',', names = True)
data_0_08 = np.genfromtxt('ss-0.08.csv', delimiter = ',', names = True)

fig, ax = plt.subplots()
ax.scatter('normal_mvmt', 'error', data=data_0_01, label='0.01', s=1)
ax.scatter('normal_mvmt', 'error', data=data_0_02, label='0.02', s=1)
ax.scatter('normal_mvmt', 'error', data=data_0_04, label='0.04', s=1)
ax.scatter('normal_mvmt', 'error', data=data_0_08, label='0.08', s=1)
ax.set_xscale('log')
ax.set_yscale('log')
ax.set_title('Error per normal movement')
ax.set_xlabel('Movement from normal change, per distance')
ax.set_ylabel('Error per distance')
ax.legend()

# plt.show()
plt.savefig('error.png')
