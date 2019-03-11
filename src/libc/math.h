#ifndef MATH_H
# define MATH_H

# define LOG22	0.30102999566

# define abs(i)		(i < 0 ? -i : i)

# define min(a, b)	(a <= b ? a : b)
# define max(a, b)	(a >= b ? a : b)

double pow(double x, double y);
double sqrt(double x);

double log(double x);
double log2(double x);

#endif
