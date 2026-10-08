"""Auto-generated setup.py."""
import os
from setuptools import setup, find_packages

HERE = os.path.abspath(os.path.dirname(__file__))
os.chdir(HERE)

setup(
    name='fastwalk',
    version='1.4.7',
    description='fastwalk',
    long_description='',
    long_description_content_type='text/markdown',
    author='',
    author_email='',
    url='',
    license='',
    packages=find_packages(exclude=('tests', 'tests.*')),
    include_package_data=True,
    python_requires='>=3.7',
    install_requires=[
    ],
    classifiers=[
        'Programming Language :: Python :: 3',
        'License :: OSI Approved :: MIT License',
        'Operating System :: OS Independent',
    ],
)
